#![allow(clippy::from_over_into)]

use crate::validate::validate_child_relative_path;
use schematic::{SchemaType, Schematic, ValidateError};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Return true of the provided file looks like a glob pattern.
pub fn is_glob_like(value: &str) -> bool {
    if value.starts_with('!') || value.contains("**") || value.contains('*') {
        return true;
    }

    if let (Some(l), Some(r)) = (value.find('{'), value.find('}')) {
        if l < r {
            return true;
        }
    }

    if let (Some(l), Some(r)) = (value.find('['), value.find(']')) {
        if l < r {
            return true;
        }
    }

    value.contains('?')
}

pub trait PortablePath: Sized {
    fn from_str(path: &str) -> Result<Self, ValidateError>;
}

macro_rules! path_type {
    ($name:ident) => {
        #[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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

        impl Schematic for $name {
            fn generate_schema() -> SchemaType {
                SchemaType::string()
            }
        }
    };
}

// Represents any file glob pattern.
path_type!(GlobPath);

impl PortablePath for GlobPath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        Ok(GlobPath(value.into()))
    }
}

// Represents any file system path.
path_type!(FilePath);

impl PortablePath for FilePath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        if is_glob_like(value) {
            return Err(ValidateError::new(
                "globs are not supported, expected a literal file path",
            ));
        }

        Ok(FilePath(value.into()))
    }
}

// Represents a project-relative file glob pattern.
path_type!(ProjectGlobPath);

impl PortablePath for ProjectGlobPath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        validate_child_relative_path(value)?;

        Ok(ProjectGlobPath(value.into()))
    }
}

// Represents a project-relative file system path.
path_type!(ProjectFilePath);

impl PortablePath for ProjectFilePath {
    fn from_str(value: &str) -> Result<Self, ValidateError> {
        if is_glob_like(value) {
            return Err(ValidateError::new(
                "globs are not supported, expected a literal file path",
            ));
        }

        validate_child_relative_path(value)?;

        Ok(ProjectFilePath(value.into()))
    }
}
