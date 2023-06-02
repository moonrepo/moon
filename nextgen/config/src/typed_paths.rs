use crate::portable_path::{
    is_glob, FilePath, GlobPath, Portable, ProjectFilePath, ProjectGlobPath,
};
use schematic::ValidateError;
use serde::{de, Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum InputPath {
    EnvVar(String),
    ProjectFile(ProjectFilePath),
    ProjectGlob(ProjectGlobPath),
    WorkspaceFile(FilePath),
    WorkspaceGlob(GlobPath),
}

impl InputPath {
    pub fn env_var(var: &str) -> InputPath {
        InputPath::EnvVar(var.to_owned())
    }

    pub fn project_file(path: &str) -> InputPath {
        InputPath::ProjectFile(ProjectFilePath(path.to_owned()))
    }

    pub fn project_glob(path: &str) -> InputPath {
        InputPath::ProjectGlob(ProjectGlobPath(path.to_owned()))
    }

    pub fn workspace_file(path: &str) -> InputPath {
        InputPath::WorkspaceFile(FilePath(path.to_owned()))
    }

    pub fn workspace_glob(path: &str) -> InputPath {
        InputPath::WorkspaceGlob(GlobPath(path.to_owned()))
    }
}

impl Portable for InputPath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        if let Some(env_var) = value.strip_prefix('$') {
            return Ok(InputPath::EnvVar(env_var.to_owned()));
        }

        if value.starts_with("/!") || value.starts_with("!/") {
            return Ok(InputPath::WorkspaceGlob(GlobPath::from_str(
                format!("!{}", &value[2..]).as_str(),
            )?));
        }

        Ok(match (value.starts_with('/'), is_glob(value)) {
            (true, true) => InputPath::WorkspaceGlob(GlobPath::from_str(&value[1..])?),
            (true, false) => InputPath::WorkspaceFile(FilePath::from_str(&value[1..])?),
            (false, true) => InputPath::ProjectGlob(ProjectGlobPath::from_str(value)?),
            (false, false) => InputPath::ProjectFile(ProjectFilePath::from_str(value)?),
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
