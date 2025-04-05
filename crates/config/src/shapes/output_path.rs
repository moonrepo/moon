#![allow(clippy::from_over_into)]

use crate::portable_path::is_glob_like;
use crate::validate::validate_child_relative_path;
use crate::{config_enum, patterns};
use moon_common::path::{
    RelativeFrom, WorkspaceRelativePathBuf, expand_to_workspace_relative, standardize_separators,
};
use schematic::{ParseError, Schema, SchemaBuilder, Schematic};
use std::cmp::Ordering;
use std::str::FromStr;

config_enum!(
    /// The different patterns a task output can be defined.
    #[serde(untagged, into = "String", try_from = "String")]
    pub enum OutputPath {
        ProjectFile(String),
        ProjectGlob(String),
        TokenFunc(String),
        TokenVar(String),
        WorkspaceFile(String),
        WorkspaceGlob(String),
    }
);

impl OutputPath {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ProjectFile(value)
            | Self::ProjectGlob(value)
            | Self::TokenFunc(value)
            | Self::TokenVar(value)
            | Self::WorkspaceFile(value)
            | Self::WorkspaceGlob(value) => value,
        }
    }

    pub fn is_glob(&self) -> bool {
        matches!(self, Self::ProjectGlob(_) | Self::WorkspaceGlob(_))
    }

    pub fn to_workspace_relative(
        &self,
        project_source: impl AsRef<str>,
    ) -> Option<WorkspaceRelativePathBuf> {
        match self {
            Self::ProjectFile(path) | Self::ProjectGlob(path) => Some(
                expand_to_workspace_relative(RelativeFrom::Project(project_source.as_ref()), path),
            ),
            Self::WorkspaceFile(path) | Self::WorkspaceGlob(path) => {
                Some(expand_to_workspace_relative(RelativeFrom::Workspace, path))
            }
            _ => None,
        }
    }
}

impl AsRef<str> for OutputPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<OutputPath> for OutputPath {
    fn as_ref(&self) -> &OutputPath {
        self
    }
}

impl PartialOrd<OutputPath> for OutputPath {
    fn partial_cmp(&self, other: &OutputPath) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OutputPath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl FromStr for OutputPath {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Token function
        if value.starts_with('@') && patterns::TOKEN_FUNC_DISTINCT.is_match(value) {
            return Ok(Self::TokenFunc(value.to_owned()));
        }

        // Token/env var
        if value.starts_with('$') {
            if patterns::ENV_VAR_DISTINCT.is_match(value) {
                return Err(ParseError::new(
                    "environment variable is not supported by itself",
                ));
            } else if patterns::ENV_VAR_GLOB_DISTINCT.is_match(value) {
                return Err(ParseError::new(
                    "environment variable globs are not supported",
                ));
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

impl TryFrom<String> for OutputPath {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for OutputPath {
    fn into(self) -> String {
        match self {
            Self::ProjectFile(value)
            | Self::ProjectGlob(value)
            | Self::TokenFunc(value)
            | Self::TokenVar(value) => value,
            Self::WorkspaceFile(path) | Self::WorkspaceGlob(path) => format!("/{path}"),
        }
    }
}

impl Schematic for OutputPath {
    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.string_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_correctly() {
        // Project relative
        assert_eq!(
            OutputPath::from_str("file.rs").unwrap(),
            OutputPath::ProjectFile("file.rs".into())
        );
        assert_eq!(
            OutputPath::from_str("dir/file.rs").unwrap(),
            OutputPath::ProjectFile("dir/file.rs".into())
        );
        assert_eq!(
            OutputPath::from_str("dir/**/*").unwrap(),
            OutputPath::ProjectGlob("dir/**/*".into())
        );
        assert_eq!(
            OutputPath::from_str("!dir/**/*").unwrap(),
            OutputPath::ProjectGlob("!dir/**/*".into())
        );
        assert_eq!(
            OutputPath::from_str("./file.rs").unwrap(),
            OutputPath::ProjectFile("file.rs".into())
        );
        assert_eq!(
            OutputPath::from_str("./dir/file.rs").unwrap(),
            OutputPath::ProjectFile("dir/file.rs".into())
        );
        assert_eq!(
            OutputPath::from_str("././dir/**/*").unwrap(),
            OutputPath::ProjectGlob("dir/**/*".into())
        );

        // Workspace relative
        assert_eq!(
            OutputPath::from_str("/file.rs").unwrap(),
            OutputPath::WorkspaceFile("file.rs".into())
        );
        assert_eq!(
            OutputPath::from_str("/dir/file.rs").unwrap(),
            OutputPath::WorkspaceFile("dir/file.rs".into())
        );
        assert_eq!(
            OutputPath::from_str("/dir/**/*").unwrap(),
            OutputPath::WorkspaceGlob("dir/**/*".into())
        );
        assert_eq!(
            OutputPath::from_str("!/dir/**/*").unwrap(),
            OutputPath::WorkspaceGlob("!dir/**/*".into())
        );
        assert_eq!(
            OutputPath::from_str("/!dir/**/*").unwrap(),
            OutputPath::WorkspaceGlob("!dir/**/*".into())
        );
    }

    #[test]
    fn parses_tokens() {
        // Functions
        assert_eq!(
            OutputPath::from_str("@group(name)").unwrap(),
            OutputPath::TokenFunc("@group(name)".into())
        );
        assert_eq!(
            OutputPath::from_str("@dirs(name)").unwrap(),
            OutputPath::TokenFunc("@dirs(name)".into())
        );
        assert_eq!(
            OutputPath::from_str("@files(name)").unwrap(),
            OutputPath::TokenFunc("@files(name)".into())
        );
        assert_eq!(
            OutputPath::from_str("@globs(name)").unwrap(),
            OutputPath::TokenFunc("@globs(name)".into())
        );
        assert_eq!(
            OutputPath::from_str("@root(name)").unwrap(),
            OutputPath::TokenFunc("@root(name)".into())
        );

        // Vars
        assert_eq!(
            OutputPath::from_str("$workspaceRoot").unwrap(),
            OutputPath::TokenVar("$workspaceRoot".into())
        );
        assert_eq!(
            OutputPath::from_str("$projectType").unwrap(),
            OutputPath::TokenVar("$projectType".into())
        );
    }

    #[test]
    #[should_panic(expected = "environment variable globs are not supported")]
    fn errors_for_env_globs() {
        OutputPath::from_str("$VAR_*").unwrap();
    }

    #[test]
    #[should_panic(expected = "parent relative paths are not supported")]
    fn errors_for_parent_relative_from_project() {
        OutputPath::from_str("../test").unwrap();
    }

    #[test]
    #[should_panic(expected = "parent relative paths are not supported")]
    fn errors_for_parent_relative_from_workspace() {
        OutputPath::from_str("/../test").unwrap();
    }
}
