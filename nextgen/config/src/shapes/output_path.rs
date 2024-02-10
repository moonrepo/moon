#![allow(clippy::from_over_into)]

use crate::patterns;
use crate::portable_path::is_glob_like;
use crate::validate::validate_child_relative_path;
use moon_common::path::{
    expand_to_workspace_relative, standardize_separators, RelativeFrom, WorkspaceRelativePathBuf,
};
use schematic::{derive_enum, SchemaType, Schematic, ValidateError};
use std::str::FromStr;

derive_enum!(
    /// The different patterns a task output can be defined.
    #[serde(untagged, into = "String", try_from = "String")]
    pub enum OutputPath {
        ProjectFile(String),
        ProjectGlob(String),
        TokenFunc(String),
        WorkspaceFile(String),
        WorkspaceGlob(String),
    }
);

impl OutputPath {
    pub fn as_str(&self) -> &str {
        match self {
            OutputPath::ProjectFile(value)
            | OutputPath::ProjectGlob(value)
            | OutputPath::TokenFunc(value)
            | OutputPath::WorkspaceFile(value)
            | OutputPath::WorkspaceGlob(value) => value,
        }
    }

    pub fn is_glob(&self) -> bool {
        matches!(
            self,
            OutputPath::ProjectGlob(_) | OutputPath::WorkspaceGlob(_)
        )
    }

    pub fn to_workspace_relative(
        &self,
        project_source: impl AsRef<str>,
    ) -> Option<WorkspaceRelativePathBuf> {
        match self {
            OutputPath::TokenFunc(_) => None,
            OutputPath::ProjectFile(path) | OutputPath::ProjectGlob(path) => Some(
                expand_to_workspace_relative(RelativeFrom::Project(project_source.as_ref()), path),
            ),
            OutputPath::WorkspaceFile(path) | OutputPath::WorkspaceGlob(path) => {
                Some(expand_to_workspace_relative(RelativeFrom::Workspace, path))
            }
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

impl FromStr for OutputPath {
    type Err = ValidateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Token function
        if value.starts_with('@') && patterns::TOKEN_FUNC_DISTINCT.is_match(value) {
            return Ok(OutputPath::TokenFunc(value.to_owned()));
        }

        // Token/env var
        if value.starts_with('$') {
            return Err(ValidateError::new(
                "token and environment variables are not supported",
            ));
        }

        let value = standardize_separators(value);

        // Workspace negated glob
        if value.starts_with("/!") || value.starts_with("!/") {
            return Ok(OutputPath::WorkspaceGlob(format!("!{}", &value[2..])));
        }

        // Workspace-relative
        if let Some(workspace_path) = value.strip_prefix('/') {
            validate_child_relative_path(workspace_path)?;

            return Ok(if is_glob_like(workspace_path) {
                OutputPath::WorkspaceGlob(workspace_path.to_owned())
            } else {
                OutputPath::WorkspaceFile(workspace_path.to_owned())
            });
        }

        // Project-relative
        let project_path = &value;

        validate_child_relative_path(project_path)?;

        Ok(if is_glob_like(project_path) {
            OutputPath::ProjectGlob(project_path.to_owned())
        } else {
            OutputPath::ProjectFile(project_path.to_owned())
        })
    }
}

impl TryFrom<String> for OutputPath {
    type Error = ValidateError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        OutputPath::from_str(&value)
    }
}

impl Into<String> for OutputPath {
    fn into(self) -> String {
        match self {
            OutputPath::ProjectFile(value)
            | OutputPath::ProjectGlob(value)
            | OutputPath::TokenFunc(value) => value,
            OutputPath::WorkspaceFile(path) | OutputPath::WorkspaceGlob(path) => format!("/{path}"),
        }
    }
}

impl Schematic for OutputPath {
    fn generate_schema() -> SchemaType {
        SchemaType::string()
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
    }

    #[test]
    #[should_panic(expected = "token and environment variables are not supported")]
    fn errors_for_env_vars() {
        OutputPath::from_str("$VAR").unwrap();
    }

    #[test]
    #[should_panic(expected = "token and environment variables are not supported")]
    fn errors_for_token_vars() {
        OutputPath::from_str("$workspaceRoot").unwrap();
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
