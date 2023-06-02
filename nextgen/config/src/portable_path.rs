use crate::validate::{validate_child_or_root_path, validate_child_relative_path};
use moon_common::path::{standardize_separators, WorkspaceRelativePathBuf};
use schemars::JsonSchema;
use schematic::ValidateError;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::path::Path;

// Not accurate at all but good enough...
pub fn is_glob(value: &str) -> bool {
    value.contains("**")
        || value.contains('*')
        || value.contains('{')
        || value.contains('[')
        || value.starts_with('!')
}

pub trait Portable: Sized {
    fn from_str(path: &str) -> Result<Self, ValidateError>;
}

macro_rules! path_type {
    ($name:ident) => {
        #[derive(Clone, Debug, Default, Eq, JsonSchema, PartialEq, Serialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl AsRef<Path> for $name {
            fn as_ref(&self) -> &Path {
                self.0.as_ref()
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                &self.0 == other
            }
        }

        impl TryFrom<String> for $name {
            type Error = ValidateError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                $name::from_str(&value)
            }
        }

        impl TryFrom<&String> for $name {
            type Error = ValidateError;

            fn try_from(value: &String) -> Result<Self, Self::Error> {
                $name::from_str(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ValidateError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                $name::from_str(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                $name::from_str(&String::deserialize(deserializer)?)
                    .map_err(|error| de::Error::custom(error.message))
            }
        }
    };
}

// Represents any file glob pattern.
path_type!(GlobPath);

impl Portable for GlobPath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        Ok(GlobPath(value.into()))
    }
}

// Represents any file system path.
path_type!(FilePath);

impl Portable for FilePath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        if is_glob(value) {
            return Err(ValidateError::new(
                "globs are not supported, expected a literal file path",
            ));
        }

        Ok(FilePath(value.into()))
    }
}

// Represents a project-relative file glob pattern.
path_type!(ProjectGlobPath);

impl Portable for ProjectGlobPath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        validate_child_relative_path(value)?;

        Ok(ProjectGlobPath(value.into()))
    }
}

// Represents a project-relative file system path.
path_type!(ProjectFilePath);

impl Portable for ProjectFilePath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        if is_glob(value) {
            return Err(ValidateError::new(
                "globs are not supported, expected a literal file path",
            ));
        }

        validate_child_relative_path(value)?;

        Ok(ProjectFilePath(value.into()))
    }
}

// Represents either a workspace or project relative glob/path, or env var.
// Workspace paths are prefixed with "/", and env vars with "$".
#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PortablePath {
    ProjectFile(FilePath),
    ProjectGlob(GlobPath),
    WorkspaceFile(FilePath),
    WorkspaceGlob(GlobPath),
}

impl PortablePath {
    /// Expand the portable path to a workspace relative path. If the path is project relative,
    /// prefix it with the provided project source. Furthermore, all paths must be standardized
    /// on "/" for path separators, as it's a requirement for globs and `RelativePathBuf`.
    pub fn to_workspace_relative(&self, project_source: &str) -> WorkspaceRelativePathBuf {
        let source = standardize_separators(project_source);
        let path = match self {
            PortablePath::ProjectFile(file) => {
                WorkspaceRelativePathBuf::from(source).join(standardize_separators(file))
            }
            PortablePath::ProjectGlob(glob) => {
                if let Some(negated_glob) = glob.0.strip_prefix('!') {
                    WorkspaceRelativePathBuf::from(format!("!{source}"))
                        .join(standardize_separators(negated_glob))
                } else {
                    WorkspaceRelativePathBuf::from(source).join(standardize_separators(glob))
                }
            }
            PortablePath::WorkspaceFile(file) => {
                WorkspaceRelativePathBuf::from(standardize_separators(file))
            }
            PortablePath::WorkspaceGlob(glob) => {
                WorkspaceRelativePathBuf::from(standardize_separators(glob))
            }
        };

        path.normalize()
    }
}

impl Portable for PortablePath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        // if let Some(env_var) = value.strip_prefix('$') {
        //     return Ok(PortablePath::EnvVar(env_var.to_owned()));
        // }

        validate_child_or_root_path(value)?;

        if value.starts_with("/!") || value.starts_with("!/") {
            return Ok(PortablePath::WorkspaceGlob(GlobPath::from_str(
                format!("!{}", &value[2..]).as_str(),
            )?));
        }

        Ok(match (value.starts_with('/'), is_glob(value)) {
            (true, true) => PortablePath::WorkspaceGlob(GlobPath::from_str(&value[1..])?),
            (true, false) => PortablePath::WorkspaceFile(FilePath::from_str(&value[1..])?),
            (false, true) => PortablePath::ProjectGlob(GlobPath::from_str(value)?),
            (false, false) => PortablePath::ProjectFile(FilePath::from_str(value)?),
        })
    }
}

impl PartialEq<&str> for PortablePath {
    fn eq(&self, other: &&str) -> bool {
        match self {
            // PortablePath::EnvVar(var) => var == other,
            PortablePath::ProjectFile(file) | PortablePath::WorkspaceFile(file) => file == other,
            PortablePath::ProjectGlob(glob) | PortablePath::WorkspaceGlob(glob) => glob == other,
        }
    }
}

impl<'de> Deserialize<'de> for PortablePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        PortablePath::from_str(&String::deserialize(deserializer)?)
            .map_err(|error| de::Error::custom(error.message))
    }
}
