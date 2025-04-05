#![allow(clippy::from_over_into)]

use crate::portable_path::is_glob_like;
use crate::validate::validate_child_relative_path;
use crate::{config_enum, patterns};
use moon_common::path::{
    RelativeFrom, WorkspaceRelativePathBuf, expand_to_workspace_relative, standardize_separators,
};
use schematic::{ParseError, Schema, SchemaBuilder, Schematic};
use std::str::FromStr;

config_enum!(
    /// The different patterns a task input can be defined.
    #[serde(untagged, into = "String", try_from = "String")]
    pub enum InputPath {
        EnvVar(String),
        EnvVarGlob(String),
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
            Self::EnvVar(value)
            | Self::EnvVarGlob(value)
            | Self::ProjectFile(value)
            | Self::ProjectGlob(value)
            | Self::TokenFunc(value)
            | Self::TokenVar(value)
            | Self::WorkspaceFile(value)
            | Self::WorkspaceGlob(value) => value,
        }
    }

    pub fn is_glob(&self) -> bool {
        matches!(
            self,
            Self::EnvVarGlob(_) | Self::ProjectGlob(_) | Self::WorkspaceGlob(_)
        )
    }

    pub fn to_workspace_relative(
        &self,
        project_source: impl AsRef<str>,
    ) -> WorkspaceRelativePathBuf {
        match self {
            Self::EnvVar(_) | Self::EnvVarGlob(_) | Self::TokenFunc(_) | Self::TokenVar(_) => {
                unreachable!()
            }
            Self::ProjectFile(path) | Self::ProjectGlob(path) => {
                expand_to_workspace_relative(RelativeFrom::Project(project_source.as_ref()), path)
            }
            Self::WorkspaceFile(path) | Self::WorkspaceGlob(path) => {
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
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Token function
        if value.starts_with('@') && patterns::TOKEN_FUNC_DISTINCT.is_match(value) {
            return Ok(Self::TokenFunc(value.to_owned()));
        }

        // Token/env var
        if let Some(var) = value.strip_prefix('$') {
            if patterns::ENV_VAR_DISTINCT.is_match(value) {
                return Ok(Self::EnvVar(var.to_owned()));
            } else if patterns::ENV_VAR_GLOB_DISTINCT.is_match(value) {
                return Ok(Self::EnvVarGlob(var.to_owned()));
            } else if patterns::TOKEN_VAR_DISTINCT.is_match(value) {
                return Ok(Self::TokenVar(value.to_owned()));
            }
        }

        let value = standardize_separators(value);

        // Workspace negated glob
        if value.starts_with("/!") || value.starts_with("!/") {
            return Ok(Self::WorkspaceGlob(format!("!{}", &value[2..])));
        }

        // Workspace-relative
        if let Some(workspace_path) = value.strip_prefix('/') {
            validate_child_relative_path(workspace_path)
                .map_err(|error| ParseError::new(error.to_string()))?;

            return Ok(if is_glob_like(workspace_path) {
                Self::WorkspaceGlob(workspace_path.to_owned())
            } else {
                Self::WorkspaceFile(workspace_path.to_owned())
            });
        }

        // Project-relative
        validate_child_relative_path(&value).map_err(|error| ParseError::new(error.to_string()))?;

        let project_path = value.trim_start_matches("./");

        Ok(if is_glob_like(project_path) {
            Self::ProjectGlob(project_path.to_owned())
        } else {
            Self::ProjectFile(project_path.to_owned())
        })
    }
}

impl TryFrom<String> for InputPath {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for InputPath {
    fn into(self) -> String {
        match self {
            Self::EnvVar(var) | Self::EnvVarGlob(var) => format!("${var}"),
            Self::ProjectFile(value)
            | Self::ProjectGlob(value)
            | Self::TokenFunc(value)
            | Self::TokenVar(value) => value,
            Self::WorkspaceFile(path) => format!("/{path}"),
            Self::WorkspaceGlob(path) => {
                if let Some(suffix) = path.strip_prefix('!') {
                    format!("!/{suffix}")
                } else {
                    format!("/{path}")
                }
            }
        }
    }
}

impl Schematic for InputPath {
    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.string_default()
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
        assert_eq!(
            InputPath::from_str("$VAR_*").unwrap(),
            InputPath::EnvVarGlob("VAR_*".into())
        );
        assert_eq!(
            InputPath::from_str("$VAR_*_SUFFIX").unwrap(),
            InputPath::EnvVarGlob("VAR_*_SUFFIX".into())
        );
        assert_eq!(
            InputPath::from_str("$*_SUFFIX").unwrap(),
            InputPath::EnvVarGlob("*_SUFFIX".into())
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
        assert_eq!(
            InputPath::from_str("./file.rs").unwrap(),
            InputPath::ProjectFile("file.rs".into())
        );
        assert_eq!(
            InputPath::from_str("././dir/file.rs").unwrap(),
            InputPath::ProjectFile("dir/file.rs".into())
        );
        assert_eq!(
            InputPath::from_str("./dir/**/*").unwrap(),
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

    #[test]
    #[should_panic(expected = "parent relative paths are not supported")]
    fn errors_for_parent_relative_from_project() {
        InputPath::from_str("../test").unwrap();
    }

    #[test]
    #[should_panic(expected = "parent relative paths are not supported")]
    fn errors_for_parent_relative_from_workspace() {
        InputPath::from_str("/../test").unwrap();
    }
}
