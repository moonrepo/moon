#![allow(clippy::from_over_into)]

use crate::portable_path::is_glob;
use crate::validate::validate_child_relative_path;
use moon_common::path::{
    expand_to_workspace_relative, standardize_separators, RelativeFrom, WorkspaceRelativePathBuf,
};
use schematic::{derive_enum, ValidateError};
use std::str::FromStr;

derive_enum!(
    #[serde(untagged, into = "String", try_from = "String")]
    pub enum InputPath {
        EnvVar(String),
        ProjectFile(String),
        ProjectGlob(String),
        WorkspaceFile(String),
        WorkspaceGlob(String),
    }
);

impl InputPath {
    pub fn as_str(&self) -> &str {
        match self {
            InputPath::EnvVar(var) => var,
            InputPath::ProjectFile(path) | InputPath::ProjectGlob(path) => path,
            InputPath::WorkspaceFile(path) | InputPath::WorkspaceGlob(path) => path,
        }
    }

    pub fn expand_to_workspace_relative(
        &self,
        project_source: impl AsRef<str>,
    ) -> WorkspaceRelativePathBuf {
        match self {
            InputPath::EnvVar(_) => unreachable!(),
            InputPath::ProjectFile(path) | InputPath::ProjectGlob(path) => {
                expand_to_workspace_relative(RelativeFrom::Project(project_source.as_ref()), path)
            }
            InputPath::WorkspaceFile(path) | InputPath::WorkspaceGlob(path) => {
                expand_to_workspace_relative(RelativeFrom::Workspace, path)
            }
        }
    }

    pub fn is_glob(&self) -> bool {
        matches!(
            self,
            InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_)
        )
    }
}

impl AsRef<str> for InputPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for InputPath {
    type Err = ValidateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = standardize_separators(value);

        // Env var
        if let Some(env_var) = value.strip_prefix('$') {
            return Ok(InputPath::EnvVar(env_var.to_owned()));
        }

        // Workspace negated glob
        if value.starts_with("/!") || value.starts_with("!/") {
            return Ok(InputPath::WorkspaceGlob(format!("!{}", &value[2..])));
        }

        // Workspace-relative
        if let Some(workspace_path) = value.strip_prefix('/') {
            validate_child_relative_path(workspace_path)?;

            return Ok(if is_glob(workspace_path) {
                InputPath::WorkspaceGlob(workspace_path.to_owned())
            } else {
                InputPath::WorkspaceFile(workspace_path.to_owned())
            });
        }

        // Project-relative
        let project_path = &value;

        validate_child_relative_path(project_path)?;

        Ok(if is_glob(project_path) {
            InputPath::ProjectGlob(project_path.to_owned())
        } else {
            InputPath::ProjectFile(project_path.to_owned())
        })
    }
}

impl TryFrom<String> for InputPath {
    type Error = ValidateError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        InputPath::from_str(&value)
    }
}

impl Into<String> for InputPath {
    fn into(self) -> String {
        match self {
            InputPath::EnvVar(var) => format!("${var}"),
            InputPath::ProjectFile(path) | InputPath::ProjectGlob(path) => path,
            InputPath::WorkspaceFile(path) | InputPath::WorkspaceGlob(path) => format!("/{path}"),
        }
    }
}
