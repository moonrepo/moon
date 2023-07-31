use crate::expander_context::{substitute_env_var, ExpanderContext};
use crate::token_expander_error::TokenExpanderError;
use moon_common::path::{self, WorkspaceRelativePathBuf};
use moon_config::{patterns, InputPath, OutputPath};
use moon_project::FileGroup;
use moon_task::Task;
use moon_time::{now_millis, now_timestamp};
use pathdiff::diff_paths;
use tracing::warn;

pub type ExpandedPaths = (Vec<WorkspaceRelativePathBuf>, Vec<WorkspaceRelativePathBuf>);

#[derive(PartialEq)]
pub enum TokenScope {
    Command,
    Args,
    Inputs,
    Outputs,
}

impl TokenScope {
    pub fn label(&self) -> String {
        match self {
            TokenScope::Command => "commands",
            TokenScope::Args => "args",
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
                let (files, globs) = self.replace_function(task, arg)?;

                for file in files {
                    args.push(handle_path(file)?);
                }

                for glob in globs {
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

    pub fn expand_inputs(&mut self, task: &Task) -> miette::Result<ExpandedPaths> {
        self.scope = TokenScope::Inputs;

        let mut files: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        for input in &task.inputs {
            match input {
                InputPath::EnvVar(_) => {
                    continue;
                }
                InputPath::TokenFunc(func) => {
                    let result = self.replace_function(task, func)?;
                    files.extend(result.0);
                    globs.extend(result.1);
                }
                InputPath::TokenVar(var) => {
                    files.push(
                        self.context
                            .project
                            .source
                            .join(self.replace_variable(task, var)?),
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
                        globs.push(file.join("**/*"));
                    } else {
                        files.push(file);
                    }
                }
                InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_) => {
                    let glob = self.replace_variables(
                        task,
                        input
                            .to_workspace_relative(&self.context.project.source)
                            .as_str(),
                    )?;

                    globs.push(WorkspaceRelativePathBuf::from(glob));
                }
            };
        }

        Ok((files, globs))
    }

    pub fn expand_outputs(&mut self, task: &Task) -> miette::Result<ExpandedPaths> {
        self.scope = TokenScope::Outputs;

        let mut files: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        for output in &task.outputs {
            match output {
                OutputPath::TokenFunc(func) => {
                    let result = self.replace_function(task, func)?;
                    files.extend(result.0);
                    globs.extend(result.1);
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
                        globs.push(path);
                    } else {
                        files.push(path);
                    }
                }
            };
        }

        Ok((files, globs))
    }

    pub fn replace_function(&self, task: &Task, value: &str) -> miette::Result<ExpandedPaths> {
        let matches = patterns::TOKEN_FUNC.captures(value).unwrap();
        let token = matches.get(0).unwrap().as_str(); // @name(arg)
        let func = matches.get(1).unwrap().as_str(); // name
        let arg = matches.get(2).unwrap().as_str(); // arg

        let mut files: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        let file_group = || -> miette::Result<&FileGroup> {
            self.check_scope(
                token,
                &[TokenScope::Args, TokenScope::Inputs, TokenScope::Outputs],
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
                files.push(
                    file_group()?
                        .root(self.context.workspace_root, &self.context.project.source)?,
                );
            }
            "dirs" => files.extend(file_group()?.dirs(self.context.workspace_root)?),
            "files" => files.extend(file_group()?.files(self.context.workspace_root)?),
            "globs" => globs.extend(file_group()?.globs()?.to_owned()),
            "group" => {
                let group = file_group()?;
                files.extend(group.files.clone());
                globs.extend(group.globs.clone());
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
                        files.push(input.to_workspace_relative(&self.context.project.source));
                    }
                    InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_) => {
                        globs.push(input.to_workspace_relative(&self.context.project.source));
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
                        files.push(
                            output
                                .to_workspace_relative(&self.context.project.source)
                                .unwrap(),
                        );
                    }
                    OutputPath::ProjectGlob(_) | OutputPath::WorkspaceGlob(_) => {
                        globs.push(
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

        Ok((files, globs))
    }

    pub fn replace_variables(&self, task: &Task, value: &str) -> miette::Result<String> {
        let mut value = value.to_owned();

        while self.has_token_variable(&value) {
            value = self.replace_variable(task, &value)?;
        }

        Ok(value)
    }

    pub fn replace_variable(&self, task: &Task, value: &str) -> miette::Result<String> {
        let Some(matches) = patterns::TOKEN_VAR.captures(value) else {
            return Ok(value.to_owned());
        };

        let token = matches.get(0).unwrap().as_str(); // $var
        let variable = matches.get(1).unwrap().as_str(); // var
        let project = self.context.project;

        self.check_scope(
            token,
            &[
                TokenScope::Command,
                TokenScope::Args,
                TokenScope::Inputs,
                TokenScope::Outputs,
            ],
        )?;

        let replaced_value = match variable {
            "workspaceRoot" => path::to_string(self.context.workspace_root)?,
            // Project
            "language" => project.language.to_string(),
            "project" => project.id.to_string(),
            "projectAlias" => project.alias.clone().unwrap_or_default(),
            "projectRoot" => path::to_string(&project.root)?,
            "projectSource" => project.source.to_string(),
            "projectType" => project.type_of.to_string(),
            // Task
            "target" => task.target.to_string(),
            "task" => task.id.to_string(),
            "taskPlatform" => task.platform.to_string(),
            "taskType" => task.type_of.to_string(),
            // Datetime
            "date" => now_timestamp().format("%F").to_string(),
            "datetime" => now_timestamp().format("%F_%T").to_string(),
            "time" => now_timestamp().format("%T").to_string(),
            "timestamp" => (now_millis() / 1000).to_string(),
            _ => {
                return Ok(value.to_owned());
            }
        };

        Ok(value.replace(token, &replaced_value))
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
