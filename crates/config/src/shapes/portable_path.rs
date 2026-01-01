#![allow(clippy::from_over_into)]

use moon_common::path::RelativePathBuf;
use schematic::{ParseError, Schema, SchemaBuilder, Schematic};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn is_glob_like(value: &str) -> bool {
    if value.starts_with('!') || value.contains("**") || value.contains('*') {
        return true;
    }

    if let (Some(l), Some(r)) = (value.find('{'), value.find('}'))
        && l < r
    {
        return true;
    }

    if let (Some(l), Some(r)) = (value.find('['), value.find(']'))
        && l < r
    {
        return true;
    }

    value.contains('?') || value.contains('|')
}

pub fn validate_relative_path(value: &str) -> Result<(), ParseError> {
    let path = Path::new(value);

    if path.has_root() || path.is_absolute() {
        return Err(ParseError::new("absolute paths are not supported"));
    }

    Ok(())
}

pub fn validate_child_relative_path(value: &str) -> Result<(), ParseError> {
    if value.contains("..") {
        return Err(ParseError::new(
            "parent directory traversal (..) is not supported",
        ));
    }

    Ok(())
}

pub trait PortablePath: Sized {
    fn parse(path: impl AsRef<str>) -> Result<Self, ParseError>;

    fn parse_relative(path: impl AsRef<str>) -> Result<Self, ParseError> {
        validate_relative_path(path.as_ref())?;

        Self::parse(path)
    }
}

macro_rules! path_type {
    ($name:ident) => {
        #[derive(Clone, Debug, Default, Deserialize, Serialize)]
        #[serde(into = "String", try_from = "String")]
        pub struct $name(pub RelativePathBuf);

        impl $name {
            pub fn to_path_buf(&self) -> PathBuf {
                PathBuf::from(self.as_str())
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0.as_str() == other.0.as_str()
            }
        }

        impl Eq for $name {}

        impl std::hash::Hash for $name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                state.write(self.as_str().as_bytes());
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl AsRef<Path> for $name {
            fn as_ref(&self) -> &Path {
                self.0.as_str().as_ref()
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                &self.0 == other
            }
        }

        impl PartialEq<&RelativePathBuf> for $name {
            fn eq(&self, other: &&RelativePathBuf) -> bool {
                &self.0 == *other
            }
        }

        impl TryFrom<String> for $name {
            type Error = ParseError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                $name::parse(&value)
            }
        }

        impl TryFrom<&String> for $name {
            type Error = ParseError;

            fn try_from(value: &String) -> Result<Self, Self::Error> {
                $name::parse(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ParseError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                $name::parse(value)
            }
        }

        impl FromStr for $name {
            type Err = ParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                $name::parse(value)
            }
        }

        impl Into<String> for $name {
            fn into(self) -> String {
                self.0.to_string()
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

        impl Deref for $name {
            type Target = RelativePathBuf;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

// Represents any glob pattern.
path_type!(GlobPath);

impl PortablePath for GlobPath {
    fn parse(value: impl AsRef<str>) -> Result<Self, ParseError> {
        let mut value = value.as_ref().to_owned();

        // Fix invalid negated workspace paths
        if value.starts_with("/!") {
            value = format!("!/{}", &value[2..]);
        }

        // Remove ./ leading parts
        let value = if let Some(suffix) = value.strip_prefix('!') {
            format!("!{}", suffix.trim_start_matches("./"))
        } else {
            value.trim_start_matches("./").to_owned()
        };

        validate_child_relative_path(&value)?;

        Ok(GlobPath(value.into()))
    }
}

// Represents any file path.
path_type!(FilePath);

impl PortablePath for FilePath {
    fn parse(value: impl AsRef<str>) -> Result<Self, ParseError> {
        let value = value.as_ref();

        validate_child_relative_path(value)?;

        if is_glob_like(value) {
            return Err(ParseError::new(
                "globs are not supported, expected a literal file path",
            ));
        }

        // Remove ./ leading parts
        Ok(FilePath(value.trim_start_matches("./").into()))
    }
}
