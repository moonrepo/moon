#![allow(clippy::from_over_into)]

use crate::validate::validate_child_relative_path;
use schemars::JsonSchema;
use schematic::ValidateError;
use serde::{Deserialize, Serialize};
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
        #[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
        #[serde(into = "String", try_from = "String")]
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

        impl Into<String> for $name {
            fn into(self) -> String {
                self.0
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
