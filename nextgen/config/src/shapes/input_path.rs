#![allow(clippy::from_over_into)]

use crate::portable_path::is_glob;
use crate::validate::validate_child_relative_path;
use moon_common::path::{
    expand_to_workspace_relative, standardize_separators, RelativeFrom, WorkspaceRelativePathBuf,
};
use schematic::{derive_enum, ValidateError};
use std::str::FromStr;

fn is_env_var(value: &str) -> bool {
    value.starts_with('$') && value == value.to_uppercase()
}

derive_enum!(
    #[serde(untagged, into = "String", try_from = "String")]
    pub enum InputPath {
        EnvVar(String),
        ProjectFile(String),
        ProjectGlob(String),
        TokenFunc(String),
        TokenVar(String),
        WorkspaceFile(String),
        WorkspaceGlob(String),
    }
);

impl InputPath {
    pub fn as_str(&self) -> &str {
        match self {
            InputPath::EnvVar(value)
            | InputPath::ProjectFile(value)
            | InputPath::ProjectGlob(value)
            | InputPath::TokenFunc(value)
            | InputPath::TokenVar(value)
            | InputPath::WorkspaceFile(value)
            | InputPath::WorkspaceGlob(value) => value,
        }
    }

    pub fn is_glob(&self) -> bool {
        matches!(
            self,
            InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_)
        )
    }

    pub fn to_workspace_relative(
        &self,
        project_source: impl AsRef<str>,
    ) -> WorkspaceRelativePathBuf {
        match self {
            InputPath::EnvVar(_) | InputPath::TokenFunc(_) | InputPath::TokenVar(_) => {
                unreachable!()
            }
            InputPath::ProjectFile(path) | InputPath::ProjectGlob(path) => {
                expand_to_workspace_relative(RelativeFrom::Project(project_source.as_ref()), path)
            }
            InputPath::WorkspaceFile(path) | InputPath::WorkspaceGlob(path) => {
                expand_to_workspace_relative(RelativeFrom::Workspace, path)
            }
        }
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
        // Token function
        if value.starts_with('@') {
            return Ok(InputPath::TokenFunc(value.to_owned()));
        }

        // Token/env var
        if let Some(var) = value.strip_prefix('$') {
            if is_env_var(value) {
                return Ok(InputPath::EnvVar(var.to_owned()));
            } else if !value.contains('/') && !value.contains('.') && !value.contains('*') {
                return Ok(InputPath::TokenVar(value.to_owned()));
            }
        }

        let value = standardize_separators(value);

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
            InputPath::ProjectFile(value)
            | InputPath::ProjectGlob(value)
            | InputPath::TokenFunc(value)
            | InputPath::TokenVar(value) => value,
            InputPath::WorkspaceFile(path) | InputPath::WorkspaceGlob(path) => format!("/{path}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_correctly() {
        assert_eq!(
            InputPath::from_str("$VAR").unwrap(),
            InputPath::EnvVar("VAR".into())
        );

        // Project relative
        assert_eq!(
            InputPath::from_str("file.rs").unwrap(),
            InputPath::ProjectFile("file.rs".into())
        );
        assert_eq!(
            InputPath::from_str("dir/file.rs").unwrap(),
            InputPath::ProjectFile("dir/file.rs".into())
        );
        assert_eq!(
            InputPath::from_str("!file.*").unwrap(),
            InputPath::ProjectGlob("!file.*".into())
        );
        assert_eq!(
            InputPath::from_str("dir/**/*").unwrap(),
            InputPath::ProjectGlob("dir/**/*".into())
        );

        // Workspace relative
        assert_eq!(
            InputPath::from_str("/file.rs").unwrap(),
            InputPath::WorkspaceFile("file.rs".into())
        );
        assert_eq!(
            InputPath::from_str("/dir/file.rs").unwrap(),
            InputPath::WorkspaceFile("dir/file.rs".into())
        );
        assert_eq!(
            InputPath::from_str("/!file.*").unwrap(),
            InputPath::WorkspaceGlob("!file.*".into())
        );
        assert_eq!(
            InputPath::from_str("!/file.*").unwrap(),
            InputPath::WorkspaceGlob("!file.*".into())
        );
        assert_eq!(
            InputPath::from_str("/dir/**/*").unwrap(),
            InputPath::WorkspaceGlob("dir/**/*".into())
        );

        // With tokens
        assert_eq!(
            InputPath::from_str("$projectSource/**/*").unwrap(),
            InputPath::ProjectGlob("$projectSource/**/*".into())
        );
        assert_eq!(
            InputPath::from_str("/.cache/$projectSource").unwrap(),
            InputPath::WorkspaceFile(".cache/$projectSource".into())
        );
    }

    #[test]
    fn parses_tokens() {
        // Functions
        assert_eq!(
            InputPath::from_str("@group(name)").unwrap(),
            InputPath::TokenFunc("@group(name)".into())
        );
        assert_eq!(
            InputPath::from_str("@dirs(name)").unwrap(),
            InputPath::TokenFunc("@dirs(name)".into())
        );
        assert_eq!(
            InputPath::from_str("@files(name)").unwrap(),
            InputPath::TokenFunc("@files(name)".into())
        );
        assert_eq!(
            InputPath::from_str("@globs(name)").unwrap(),
            InputPath::TokenFunc("@globs(name)".into())
        );
        assert_eq!(
            InputPath::from_str("@root(name)").unwrap(),
            InputPath::TokenFunc("@root(name)".into())
        );

        // Vars
        assert_eq!(
            InputPath::from_str("$workspaceRoot").unwrap(),
            InputPath::TokenVar("$workspaceRoot".into())
        );
        assert_eq!(
            InputPath::from_str("$projectType").unwrap(),
            InputPath::TokenVar("$projectType".into())
        );
    }
}
