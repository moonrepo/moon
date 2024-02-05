use crate::expander_context::{substitute_env_var, ExpanderContext};
use crate::token_expander_error::TokenExpanderError;
use moon_common::path::{self, WorkspaceRelativePathBuf};
use moon_config::{patterns, InputPath, OutputPath};
use moon_project::FileGroup;
use moon_task::Task;
use moon_time::{now_millis, now_timestamp};
use pathdiff::diff_paths;
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use tracing::warn;

#[derive(Debug, Default, PartialEq)]
pub struct ExpandedResult {
    pub env: Vec<String>,
    pub files: Vec<WorkspaceRelativePathBuf>,
    pub globs: Vec<WorkspaceRelativePathBuf>,
}

#[derive(PartialEq)]
pub enum TokenScope {
    Command,
    Args,
    Env,
    Inputs,
    Outputs,
}

impl TokenScope {
    pub fn label(&self) -> String {
        match self {
            TokenScope::Command => "commands",
            TokenScope::Args => "args",
            TokenScope::Env => "env",
            TokenScope::Inputs => "inputs",
            TokenScope::Outputs => "outputs",
        }
        .into()
    }
}

pub struct TokenExpander<'graph, 'query> {
    context: &'graph ExpanderContext<'graph, 'query>,
    pub scope: TokenScope,
}

impl<'graph, 'query> TokenExpander<'graph, 'query> {
    pub fn new(context: &'graph ExpanderContext<'graph, 'query>) -> Self {
        Self {
            scope: TokenScope::Args,
            context,
        }
    }

    pub fn has_token_function(&self, value: &str) -> bool {
        if value.contains('@') {
            if patterns::TOKEN_FUNC_DISTINCT.is_match(value) {
                return true;
            } else if patterns::TOKEN_FUNC.is_match(value) {
                warn!(
                    "Found a token function in `{}` with other content. Token functions *must* be used literally as the only value.",
                    value
                );
            }
        }

        false
    }

    pub fn has_token_variable(&self, value: &str) -> bool {
        value.contains('$') && patterns::TOKEN_VAR.is_match(value)
    }

    pub fn expand_command(&mut self, task: &Task) -> miette::Result<String> {
        self.scope = TokenScope::Command;

        if self.has_token_function(&task.command) {
            // Trigger the scope error
            self.replace_function(task, &task.command)?;
        }

        self.replace_variables(task, &task.command)
    }

    pub fn expand_args(&mut self, task: &Task) -> miette::Result<Vec<String>> {
        self.scope = TokenScope::Args;

        let mut args = vec![];

        let handle_path = |path: WorkspaceRelativePathBuf| -> miette::Result<String> {
            // From workspace root to any file
            if task.options.run_from_workspace_root {
                Ok(format!("./{}", path))

                // From project root to project file
            } else if let Ok(proj_path) = path.strip_prefix(&self.context.project.source) {
                Ok(format!("./{}", proj_path))

                // From project root to non-project file
            } else {
                let abs_path = path.to_logical_path(self.context.workspace_root);

                path::to_virtual_string(
                    diff_paths(&abs_path, &self.context.project.root).unwrap_or(abs_path),
                )
            }
        };

        for arg in &task.args {
            // Token functions
            if self.has_token_function(arg) {
                let result = self.replace_function(task, arg)?;

                for file in result.files {
                    args.push(handle_path(file)?);
                }

                for glob in result.globs {
                    args.push(handle_path(glob)?);
                }

            // Token variables
            } else if self.has_token_variable(arg) {
                args.push(self.replace_variables(task, arg)?);

            // Environment variables
            } else if patterns::ENV_VAR_SUBSTITUTE.is_match(arg) {
                args.push(substitute_env_var(arg, &task.env));

            // Normal arg
            } else {
                args.push(arg.to_owned());
            }
        }

        Ok(args)
    }

    pub fn expand_env(&mut self, task: &Task) -> miette::Result<FxHashMap<String, String>> {
        self.scope = TokenScope::Env;

        let mut env = FxHashMap::default();

        for (key, value) in &task.env {
            if self.has_token_function(value) {
                let result = self.replace_function(task, value)?;

                let mut list = vec![];
                list.extend(result.files);
                list.extend(result.globs);

                env.insert(
                    key.to_owned(),
                    list.iter()
                        .map(|i| i.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                );
            } else if self.has_token_variable(value) {
                env.insert(key.to_owned(), self.replace_variables(task, value)?);
            } else {
                env.insert(key.to_owned(), value.to_owned());
            }
        }

        Ok(env)
    }

    pub fn expand_inputs(&mut self, task: &Task) -> miette::Result<ExpandedResult> {
        self.scope = TokenScope::Inputs;

        let mut result = ExpandedResult::default();

        for input in &task.inputs {
            match input {
                InputPath::EnvVar(var) => {
                    result.env.push(var.to_owned());
                }
                InputPath::TokenFunc(func) => {
                    let inner_result = self.replace_function(task, func)?;

                    result.env.extend(inner_result.env);
                    result.files.extend(inner_result.files);
                    result.globs.extend(inner_result.globs);
                }
                InputPath::TokenVar(var) => {
                    result.files.push(
                        self.context
                            .project
                            .source
                            .join(self.replace_variable(task, Cow::Borrowed(var))?.as_ref()),
                    );
                }
                InputPath::ProjectFile(_) | InputPath::WorkspaceFile(_) => {
                    let file = WorkspaceRelativePathBuf::from(
                        self.replace_variables(
                            task,
                            input
                                .to_workspace_relative(&self.context.project.source)
                                .as_str(),
                        )?,
                    );
                    let abs_file = file.to_path(self.context.workspace_root);

                    // This is a special case that converts "foo" to "foo/**/*",
                    // when the input is a directory. This is necessary for VCS hashing.
                    if abs_file.exists() && abs_file.is_dir() {
                        result.globs.push(file.join("**/*"));
                    } else {
                        result.files.push(file);
                    }
                }
                InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_) => {
                    let glob = self.replace_variables(
                        task,
                        input
                            .to_workspace_relative(&self.context.project.source)
                            .as_str(),
                    )?;

                    result.globs.push(WorkspaceRelativePathBuf::from(glob));
                }
            };
        }

        Ok(result)
    }

    pub fn expand_outputs(&mut self, task: &Task) -> miette::Result<ExpandedResult> {
        self.scope = TokenScope::Outputs;

        let mut result = ExpandedResult::default();

        for output in &task.outputs {
            match output {
                OutputPath::TokenFunc(func) => {
                    let inner_result = self.replace_function(task, func)?;

                    result.files.extend(inner_result.files);
                    result.globs.extend(inner_result.globs);
                }
                _ => {
                    let path = WorkspaceRelativePathBuf::from(
                        self.replace_variables(
                            task,
                            output
                                .to_workspace_relative(&self.context.project.source)
                                .unwrap()
                                .as_str(),
                        )?,
                    );

                    if output.is_glob() {
                        result.globs.push(path);
                    } else {
                        result.files.push(path);
                    }
                }
            };
        }

        Ok(result)
    }

    pub fn replace_function(&self, task: &Task, value: &str) -> miette::Result<ExpandedResult> {
        let matches = patterns::TOKEN_FUNC.captures(value).unwrap();
        let token = matches.get(0).unwrap().as_str(); // @name(arg)
        let func = matches.get(1).unwrap().as_str(); // name
        let arg = matches.get(2).unwrap().as_str(); // arg
        let mut result = ExpandedResult::default();

        let loose_check = matches!(self.scope, TokenScope::Outputs);
        let file_group = || -> miette::Result<&FileGroup> {
            self.check_scope(
                token,
                &[
                    TokenScope::Args,
                    TokenScope::Env,
                    TokenScope::Inputs,
                    TokenScope::Outputs,
                ],
            )?;

            Ok(self.context.project.file_groups.get(arg).ok_or_else(|| {
                TokenExpanderError::UnknownFileGroup {
                    group: arg.to_owned(),
                    token: token.to_owned(),
                }
            })?)
        };

        match func {
            // File groups
            "root" => {
                result.files.push(
                    file_group()?
                        .root(self.context.workspace_root, &self.context.project.source)?,
                );
            }
            "dirs" => {
                result
                    .files
                    .extend(file_group()?.dirs(self.context.workspace_root, loose_check)?);
            }
            "files" => {
                result
                    .files
                    .extend(file_group()?.files(self.context.workspace_root, loose_check)?);
            }
            "globs" => {
                result.globs.extend(file_group()?.globs()?.to_owned());
            }
            "group" => {
                let group = file_group()?;
                result.files.extend(group.files.clone());
                result.globs.extend(group.globs.clone());

                // Only inputs can use env vars, but instead of failing for other
                // scopes, just ignore them
                if self.scope == TokenScope::Inputs {
                    result.env.extend(group.env.clone());
                }
            }
            "env" => {
                self.check_scope(token, &[TokenScope::Inputs])?;

                result.env.extend(file_group()?.env()?.to_owned());
            }
            // Inputs, outputs
            "in" => {
                self.check_scope(token, &[TokenScope::Args])?;

                let index = self.parse_index(token, arg)?;
                let input =
                    task.inputs
                        .get(index)
                        .ok_or_else(|| TokenExpanderError::MissingInIndex {
                            index,
                            token: token.to_owned(),
                        })?;

                match input {
                    InputPath::ProjectFile(_) | InputPath::WorkspaceFile(_) => {
                        result
                            .files
                            .push(input.to_workspace_relative(&self.context.project.source));
                    }
                    InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_) => {
                        result
                            .globs
                            .push(input.to_workspace_relative(&self.context.project.source));
                    }
                    _ => {
                        return Err(TokenExpanderError::InvalidTokenIndexReference {
                            token: token.to_owned(),
                        }
                        .into())
                    }
                };
            }
            "out" => {
                self.check_scope(token, &[TokenScope::Args])?;

                let index = self.parse_index(token, arg)?;
                let output =
                    task.outputs
                        .get(index)
                        .ok_or_else(|| TokenExpanderError::MissingOutIndex {
                            index,
                            token: token.to_owned(),
                        })?;

                match output {
                    OutputPath::ProjectFile(_) | OutputPath::WorkspaceFile(_) => {
                        result.files.push(
                            output
                                .to_workspace_relative(&self.context.project.source)
                                .unwrap(),
                        );
                    }
                    OutputPath::ProjectGlob(_) | OutputPath::WorkspaceGlob(_) => {
                        result.globs.push(
                            output
                                .to_workspace_relative(&self.context.project.source)
                                .unwrap(),
                        );
                    }
                    _ => {
                        return Err(TokenExpanderError::InvalidTokenIndexReference {
                            token: token.to_owned(),
                        }
                        .into())
                    }
                };
            }
            _ => {
                return Err(TokenExpanderError::UnknownToken {
                    token: token.to_owned(),
                }
                .into())
            }
        };

        Ok(result)
    }

    pub fn replace_variables(&self, task: &Task, value: &str) -> miette::Result<String> {
        let mut value = Cow::Borrowed(value);

        while self.has_token_variable(&value) {
            value = self.replace_variable(task, value)?;
        }

        Ok(value.to_string())
    }

    pub fn replace_variable<'l>(
        &self,
        task: &Task,
        value: Cow<'l, str>,
    ) -> miette::Result<Cow<'l, str>> {
        let Some(matches) = patterns::TOKEN_VAR.captures(&value) else {
            return Ok(value);
        };

        let token = matches.get(0).unwrap().as_str(); // $var
        let variable = matches.get(1).unwrap().as_str(); // var
        let project = self.context.project;

        self.check_scope(
            token,
            &[
                TokenScope::Command,
                TokenScope::Args,
                TokenScope::Env,
                TokenScope::Inputs,
                TokenScope::Outputs,
            ],
        )?;

        let replaced_value = match variable {
            "workspaceRoot" => Cow::Owned(path::to_string(self.context.workspace_root)?),
            // Project
            "language" => Cow::Owned(project.language.to_string()),
            "project" => Cow::Borrowed(project.id.as_str()),
            "projectAlias" => match project.alias.as_ref() {
                Some(alias) => Cow::Borrowed(alias.as_str()),
                None => Cow::Owned(String::new()),
            },
            "projectRoot" => Cow::Owned(path::to_string(&project.root)?),
            "projectSource" => Cow::Borrowed(project.source.as_str()),
            "projectType" => Cow::Owned(project.type_of.to_string()),
            // Task
            "target" => Cow::Borrowed(task.target.as_str()),
            "task" => Cow::Borrowed(task.id.as_str()),
            "taskPlatform" => Cow::Owned(task.platform.to_string()),
            "taskType" => Cow::Owned(task.type_of.to_string()),
            // Datetime
            "date" => Cow::Owned(now_timestamp().format("%F").to_string()),
            "datetime" => Cow::Owned(now_timestamp().format("%F_%T").to_string()),
            "time" => Cow::Owned(now_timestamp().format("%T").to_string()),
            "timestamp" => Cow::Owned((now_millis() / 1000).to_string()),
            _ => {
                return Ok(value);
            }
        };

        Ok(value.replace(token, &replaced_value).into())
    }

    fn check_scope(&self, token: &str, allowed: &[TokenScope]) -> miette::Result<()> {
        if !allowed.contains(&self.scope) {
            return Err(TokenExpanderError::InvalidTokenScope {
                token: token.to_owned(),
                scope: self.scope.label(),
            }
            .into());
        }

        Ok(())
    }

    fn parse_index(&self, token: &str, value: &str) -> miette::Result<usize> {
        Ok(value
            .parse::<usize>()
            .map_err(|_| TokenExpanderError::InvalidTokenIndex {
                token: token.to_owned(),
                index: value.to_owned(),
            })?)
    }
}
