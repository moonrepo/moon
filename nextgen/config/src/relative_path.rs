use crate::validate::{validate_child_or_root_path, validate_child_relative_path};
use schematic::ValidateError;
use serde::{de, Deserialize, Deserializer, Serialize};

// Not accurate at all but good enough...
fn is_glob(value: &str) -> bool {
    value.contains("**") || value.contains("*") || value.contains("{") || value.contains("[")
}

pub trait FromPathStr: Sized {
    fn from_path_str(path: &str) -> Result<Self, ValidateError>;
}

macro_rules! path_type {
    ($name:ident) => {
        #[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
        pub struct $name(pub String);

        impl TryFrom<&str> for $name {
            type Error = ValidateError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                $name::from_path_str(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;

                $name::from_path_str(&value).map_err(|error| de::Error::custom(error.message))
            }
        }
    };
}

// Represents a file glob pattern.
path_type!(GlobPath);

impl FromPathStr for GlobPath {
    fn from_path_str(value: &str) -> Result<Self, ValidateError> {
        Ok(GlobPath(value.into()))
    }
}

// Represents a file system path.
path_type!(FilePath);

impl FromPathStr for FilePath {
    fn from_path_str(value: &str) -> Result<Self, ValidateError> {
        if is_glob(value) {
            return Err(ValidateError::new(
                "globs are not supported, expected a literal file path",
            ));
        }

        Ok(FilePath(value.into()))
    }
}

// Represents a valid child/project relative file system path.
// Will fail on absolute paths ("/") and parent relative paths ("../").
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectRelativePath<T: FromPathStr>(pub T);

impl<T: FromPathStr> FromPathStr for ProjectRelativePath<T> {
    fn from_path_str(value: &str) -> Result<Self, ValidateError> {
        validate_child_relative_path(value)?;

        if value.starts_with("/") {
            return Err(ValidateError::new(
                "workspace relative paths are not supported",
            ));
        }

        let value = T::from_path_str(value)?;

        Ok(ProjectRelativePath(value))
    }
}

impl<'de, T: FromPathStr> Deserialize<'de> for ProjectRelativePath<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let path = String::deserialize(deserializer)?;

        ProjectRelativePath::from_path_str(&path).map_err(|error| de::Error::custom(error.message))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum RelativePath {
    ProjectFile(FilePath),
    ProjectGlob(GlobPath),
    WorkspaceFile(FilePath),
    WorkspaceGlob(GlobPath),
}

impl FromPathStr for RelativePath {
    fn from_path_str(value: &str) -> Result<Self, ValidateError> {
        validate_child_or_root_path(value)?;

        Ok(match (value.starts_with('/'), is_glob(value)) {
            (true, true) => RelativePath::WorkspaceGlob(GlobPath::from_path_str(&value[1..])?),
            (true, false) => RelativePath::WorkspaceFile(FilePath::from_path_str(&value[1..])?),
            (false, true) => RelativePath::ProjectGlob(GlobPath::from_path_str(&value)?),
            (false, false) => RelativePath::ProjectFile(FilePath::from_path_str(&value)?),
        })
    }
}

impl<'de> Deserialize<'de> for RelativePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        RelativePath::from_path_str(&value).map_err(|error| de::Error::custom(error.message))
    }
}
