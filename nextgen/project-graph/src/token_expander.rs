use crate::project_graph_error::ProjectGraphError;
use moon_common::path::{self, WorkspaceRelativePathBuf};
use moon_config::{InputPath, OutputPath};
use moon_project::{FileGroup, Project};
use moon_task::Task;
use moon_time::{now_millis, now_timestamp};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;
use tracing::warn;

pub static TOKEN_GROUP: &str = "([0-9A-Za-z_-]+)";

pub static TOKEN_FUNC_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(format!("^@([a-z]+)\\({}\\)$", TOKEN_GROUP).as_str()).unwrap());

pub static TOKEN_FUNC_ANYWHERE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(format!("@([a-z]+)\\({}\\)", TOKEN_GROUP).as_str()).unwrap());

pub static TOKEN_VAR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new("\\$(language|projectAlias|projectRoot|projectSource|projectType|project|target|taskPlatform|taskType|task|workspaceRoot|timestamp|datetime|date|time)").unwrap()
});

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

pub struct TokenExpander<'graph> {
    pub scope: TokenScope,
    pub project: &'graph Project,
    pub task: &'graph Task,
    pub workspace_root: &'graph Path,
}

impl<'graph> TokenExpander<'graph> {
    pub fn for_command(
        project: &'graph Project,
        task: &'graph Task,
        workspace_root: &'graph Path,
    ) -> Self {
        Self {
            scope: TokenScope::Command,
            project,
            task,
            workspace_root,
        }
    }

    pub fn for_args(
        project: &'graph Project,
        task: &'graph Task,
        workspace_root: &'graph Path,
    ) -> Self {
        Self {
            scope: TokenScope::Args,
            project,
            task,
            workspace_root,
        }
    }

    pub fn for_inputs(
        project: &'graph Project,
        task: &'graph Task,
        workspace_root: &'graph Path,
    ) -> Self {
        Self {
            scope: TokenScope::Inputs,
            project,
            task,
            workspace_root,
        }
    }

    pub fn for_outputs(
        project: &'graph Project,
        task: &'graph Task,
        workspace_root: &'graph Path,
    ) -> Self {
        Self {
            scope: TokenScope::Outputs,
            project,
            task,
            workspace_root,
        }
    }

    pub fn has_token_function(&self, value: &str) -> bool {
        if value.contains('@') {
            if TOKEN_FUNC_PATTERN.is_match(value) {
                return true;
            } else if TOKEN_FUNC_ANYWHERE_PATTERN.is_match(value) {
                warn!(
                    "Found a token function in `{}` with other content. Token functions *must* be used literally as the only value.",
                    value
                );
            }
        }

        false
    }

    pub fn has_token_variable(&self, value: &str) -> bool {
        value.contains('$') && TOKEN_VAR_PATTERN.is_match(&value)
    }

    pub fn expand_command(&self) -> miette::Result<String> {
        if self.has_token_function(&self.task.command) {
            // Trigger the scope error
            self.replace_function(&self.task.command)?;
        }

        self.replace_variables(&self.task.command)
    }

    pub fn expand_inputs(&self) -> miette::Result<ExpandedPaths> {
        let mut files: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        for input in &self.task.inputs {
            match input {
                InputPath::EnvVar(_) => {
                    continue;
                }
                InputPath::TokenFunc(func) => {
                    let result = self.replace_function(&func)?;
                    files.extend(result.0);
                    globs.extend(result.1);
                }
                InputPath::TokenVar(var) => {
                    files.push(self.project.source.join(self.replace_variable(var)?));
                }
                InputPath::ProjectFile(_) | InputPath::WorkspaceFile(_) => {
                    let file = WorkspaceRelativePathBuf::from(self.replace_variables(
                        input.to_workspace_relative(&self.project.source).as_str(),
                    )?);
                    let abs_file = file.to_path(self.workspace_root);

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
                        input.to_workspace_relative(&self.project.source).as_str(),
                    )?;

                    globs.push(WorkspaceRelativePathBuf::from(glob));
                }
            };
        }

        Ok((files, globs))
    }

    pub fn expand_outputs(&self) -> miette::Result<ExpandedPaths> {
        let mut files: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        for output in &self.task.outputs {
            match output {
                OutputPath::TokenFunc(func) => {
                    let result = self.replace_function(&func)?;
                    files.extend(result.0);
                    globs.extend(result.1);
                }
                _ => {
                    let path = WorkspaceRelativePathBuf::from(
                        self.replace_variables(
                            output
                                .to_workspace_relative(&self.project.source)
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

    pub fn replace_function(&self, value: &str) -> miette::Result<ExpandedPaths> {
        let matches = TOKEN_FUNC_PATTERN.captures(value).unwrap();
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

            Ok(self.project.file_groups.get(arg).ok_or_else(|| {
                ProjectGraphError::UnknownFileGroup {
                    group: arg.to_owned(),
                    token: token.to_owned(),
                }
            })?)
        };

        match func {
            // File groups
            "root" => {
                files.push(file_group()?.root(self.workspace_root, &self.project.source)?);
            }
            "dirs" => files.extend(file_group()?.dirs(self.workspace_root)?),
            "files" => files.extend(file_group()?.files(self.workspace_root)?),
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
                let input = self.task.inputs.get(index).ok_or_else(|| {
                    ProjectGraphError::MissingInIndex {
                        index,
                        token: token.to_owned(),
                    }
                })?;

                match input {
                    InputPath::ProjectFile(_) | InputPath::WorkspaceFile(_) => {
                        files.push(input.to_workspace_relative(&self.project.source));
                    }
                    InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_) => {
                        globs.push(input.to_workspace_relative(&self.project.source));
                    }
                    _ => {
                        return Err(ProjectGraphError::InvalidTokenIndexReference {
                            token: token.to_owned(),
                        }
                        .into())
                    }
                };
            }
            "out" => {
                self.check_scope(token, &[TokenScope::Args])?;

                let index = self.parse_index(token, arg)?;
                let output = self.task.outputs.get(index).ok_or_else(|| {
                    ProjectGraphError::MissingOutIndex {
                        index,
                        token: token.to_owned(),
                    }
                })?;

                match output {
                    OutputPath::ProjectFile(_) | OutputPath::WorkspaceFile(_) => {
                        files.push(output.to_workspace_relative(&self.project.source).unwrap());
                    }
                    OutputPath::ProjectGlob(_) | OutputPath::WorkspaceGlob(_) => {
                        globs.push(output.to_workspace_relative(&self.project.source).unwrap());
                    }
                    _ => {
                        return Err(ProjectGraphError::InvalidTokenIndexReference {
                            token: token.to_owned(),
                        }
                        .into())
                    }
                };
            }
            _ => {
                return Err(ProjectGraphError::UnknownToken {
                    token: token.to_owned(),
                }
                .into())
            }
        };

        Ok((files, globs))
    }

    pub fn replace_variables(&self, value: &str) -> miette::Result<String> {
        let mut value = value.to_owned();

        while self.has_token_variable(&value) {
            value = self.replace_variable(&value)?;
        }

        Ok(value)
    }

    pub fn replace_variable(&self, value: &str) -> miette::Result<String> {
        let Some(matches) = TOKEN_VAR_PATTERN.captures(value) else {
            return Ok(value.to_owned());
        };

        let token = matches.get(0).unwrap().as_str(); // $var
        let variable = matches.get(1).unwrap().as_str(); // var
        let project = self.project;
        let task = self.task;

        self.check_scope(
            token,
            &[TokenScope::Command, TokenScope::Args, TokenScope::Inputs],
        )?;

        let replaced_value = match variable {
            "workspaceRoot" => path::to_string(self.workspace_root)?,
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
            return Err(ProjectGraphError::InvalidTokenScope {
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
            .map_err(|_| ProjectGraphError::InvalidTokenIndex {
                token: token.to_owned(),
                index: value.to_owned(),
            })?)
    }
}
