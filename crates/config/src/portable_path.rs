#![allow(clippy::from_over_into)]

use crate::validate::validate_child_relative_path;
use schematic::{ParseError, Schema, SchemaBuilder, Schematic};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

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

pub trait PortablePath: FromStr {
    fn parse(path: &str) -> Result<Self, Self::Err> {
        Self::from_str(path)
    }
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
            type Error = ParseError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                $name::from_str(&value)
            }
        }

        impl TryFrom<&String> for $name {
            type Error = ParseError;

            fn try_from(value: &String) -> Result<Self, Self::Error> {
                $name::from_str(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ParseError;

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
            fn build_schema(mut schema: SchemaBuilder) -> Schema {
                schema.string_default()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

// Represents any file glob pattern.
path_type!(GlobPath);

impl PortablePath for GlobPath {}

impl FromStr for GlobPath {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, ParseError> {
        Ok(GlobPath(value.into()))
    }
}

// Represents any file system path.
path_type!(FilePath);

impl PortablePath for FilePath {}

impl FromStr for FilePath {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, ParseError> {
        if is_glob_like(value) {
            return Err(ParseError::new(
                "globs are not supported, expected a literal file path",
            ));
        }

        Ok(FilePath(value.into()))
    }
}

// Represents a project-relative file glob pattern.
path_type!(ProjectGlobPath);

impl PortablePath for ProjectGlobPath {}

impl FromStr for ProjectGlobPath {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, ParseError> {
        validate_child_relative_path(value).map_err(|error| ParseError::new(error.to_string()))?;

        Ok(ProjectGlobPath(value.into()))
    }
}

// Represents a project-relative file system path.
path_type!(ProjectFilePath);

impl PortablePath for ProjectFilePath {}

impl FromStr for ProjectFilePath {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, ParseError> {
        if is_glob_like(value) {
            return Err(ParseError::new(
                "globs are not supported, expected a literal file path",
            ));
        }

        validate_child_relative_path(value).map_err(|error| ParseError::new(error.to_string()))?;

        Ok(ProjectFilePath(value.into()))
    }
}
