use crate::portable_path::{is_glob, Portable};
use crate::validate::validate_child_relative_path;
use moon_common::path::standardize_separators;
use schematic::ValidateError;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Eq, PartialEq, schemars::JsonSchema)]
#[serde(untagged)]
pub enum InputPath {
    EnvVar(String),
    ProjectFile(String),
    ProjectGlob(String),
    WorkspaceFile(String),
    WorkspaceGlob(String),
}

impl InputPath {
    pub fn env_var(var: &str) -> InputPath {
        InputPath::EnvVar(var.to_owned())
    }

    pub fn project_file(path: &str) -> InputPath {
        InputPath::ProjectFile(path.to_owned())
    }

    pub fn project_glob(path: &str) -> InputPath {
        InputPath::ProjectGlob(path.to_owned())
    }

    pub fn workspace_file(path: &str) -> InputPath {
        InputPath::WorkspaceFile(path.to_owned())
    }

    pub fn workspace_glob(path: &str) -> InputPath {
        InputPath::WorkspaceGlob(path.to_owned())
    }
}

impl Portable for InputPath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
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

impl<'de> Deserialize<'de> for InputPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        InputPath::from_str(&String::deserialize(deserializer)?)
            .map_err(|error| de::Error::custom(error.message))
    }
}

impl Serialize for InputPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            InputPath::EnvVar(var) => format!("${var}"),
            InputPath::ProjectFile(path) | InputPath::ProjectGlob(path) => path.to_owned(),
            InputPath::WorkspaceFile(path) | InputPath::WorkspaceGlob(path) => format!("/{path}"),
        };

        serializer.serialize_str(&value)
    }
}
