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

    // Check for brace patterns, but exclude environment variable syntax
    // We need to check ALL brace pairs, not just the first one, because a path
    // like .env.${VAR}.{a,b} has both an env var and a glob pattern
    let mut search_from = 0;
    while let Some(l) = value[search_from..].find('{') {
        let l = search_from + l;
        if let Some(r_offset) = value[l..].find('}') {
            let r = l + r_offset;

            // Check if this is an environment variable: ${VAR} or ${VAR:-default}
            // Environment variables have $ immediately before the {
            if l > 0 && value.as_bytes().get(l - 1) == Some(&b'$') {
                // This is likely an env var like ${VAR}, check the contents
                let inside = &value[l + 1..r];

                // If it contains comma or .. it's a glob {a,b} or {a..z}
                // If it's alphanumeric/underscore with optional flags, it's an env var
                if inside.contains(',') || inside.contains("..") {
                    return true;
                }

                // Otherwise, it's an env var, not a glob - continue to next brace pair
            } else {
                // No $ before {, this is a glob pattern like {a,b}
                return true;
            }

            // Move search position past this brace pair
            search_from = r + 1;
        } else {
            // No matching closing brace, stop searching
            break;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_glob_like_distinguishes_env_vars_from_globs() {
        // Environment variables should NOT be detected as globs
        assert!(!is_glob_like(".env.${NODE_ENV}"));
        assert!(!is_glob_like(".env.${NODE_ENV:-production}"));
        assert!(!is_glob_like("$HOME/.env"));
        assert!(!is_glob_like("${HOME}/.env"));
        assert!(!is_glob_like(".env.${VAR1}.${VAR2}"));

        // Actual globs should still be detected
        assert!(is_glob_like("*.js"));
        assert!(is_glob_like("**/*.ts"));
        assert!(is_glob_like("config.{js,ts}"));
        assert!(is_glob_like("file{1..10}.txt"));
        assert!(is_glob_like("test-?.js"));
        assert!(is_glob_like("[abc]*.txt"));
        assert!(is_glob_like("a|b"));

        // Edge case: env var that contains comma (should be detected as glob)
        assert!(is_glob_like("${VAR,OTHER}"));
        assert!(is_glob_like("${VAR..OTHER}"));
    }

    #[test]
    fn test_is_glob_like_multiple_brace_pairs() {
        // Multiple env vars should NOT be detected as globs
        assert!(!is_glob_like(".env.${VAR1}.${VAR2}"));
        assert!(!is_glob_like("${HOME}/.env.${NODE_ENV}"));
        assert!(!is_glob_like("${VAR1}/${VAR2}/${VAR3}"));

        // First brace is env var, second is glob - should be detected as glob
        assert!(is_glob_like(".env.${VAR}.{a,b}"));
        assert!(is_glob_like("${HOME}/config.{js,ts}"));
        assert!(is_glob_like(".env.${NODE_ENV}.file{1..10}.txt"));

        // First brace is glob, second is env var - should be detected as glob
        assert!(is_glob_like("config.{js,ts}.${VAR}"));
        assert!(is_glob_like("{a,b}/${HOME}/file"));
    }

    #[test]
    fn test_filepath_parse_accepts_env_vars() {
        // These should now be accepted
        assert!(FilePath::parse(".env.${NODE_ENV}").is_ok());
        assert!(FilePath::parse(".env.$NODE_ENV").is_ok());
        assert!(FilePath::parse("${HOME}/.env").is_ok());
        assert!(FilePath::parse(".env.${VAR:-default}").is_ok());

        // Globs should still be rejected
        assert!(FilePath::parse("*.js").is_err());
        assert!(FilePath::parse("config.{js,ts}").is_err());
    }
}
