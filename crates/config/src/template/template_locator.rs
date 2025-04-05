use crate::is_glob_like;
use crate::portable_path::{FilePath, GlobPath};
use regex::Regex;
use schematic::{ParseError, Schema, SchemaBuilder, Schematic, derive_enum};
use semver::Version;
use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

static GIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^(?<url>[a-zA-Z@0-9.-]+/[a-zA-Z0-9-_./]+)#(?<revision>[a-z0-9-_.@]+)$").unwrap()
});

static NPM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^(?<package>(@[a-z][a-z0-9-_.]*/)?[a-z][a-z0-9-_.]*)#(?<version>[a-z0-9-.+]+)$")
        .unwrap()
});

derive_enum!(
    #[serde(untagged, try_from = "String", into = "String")]
    pub enum TemplateLocator {
        File {
            path: FilePath,
        },
        Glob {
            glob: GlobPath,
        },
        Git {
            remote_url: String,
            revision: String,
        },
        Npm {
            package: String,
            version: Version,
        },
    }
);

impl fmt::Display for TemplateLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateLocator::File { path } => write!(f, "file://{path}"),
            TemplateLocator::Glob { glob } => write!(f, "glob://{glob}"),
            TemplateLocator::Git {
                remote_url,
                revision,
            } => write!(f, "git://{remote_url}#{revision}"),
            TemplateLocator::Npm { package, version } => write!(f, "npm://{package}#{version}"),
        }
    }
}

impl Schematic for TemplateLocator {
    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.string_default()
    }
}

impl FromStr for TemplateLocator {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(index) = value.find(':') {
            let protocol = &value[0..index];
            let mut inner_value = &value[index + 1..];

            if inner_value.starts_with("//") {
                inner_value = &value[index + 3..];
            }

            match protocol {
                "git" | "git+http" | "git+https" => {
                    if let Some(result) = GIT.captures(inner_value) {
                        return Ok(TemplateLocator::Git {
                            remote_url: result.name("url").unwrap().as_str().to_owned(),
                            revision: result.name("revision").unwrap().as_str().to_owned(),
                        });
                    }

                    return Err(ParseError::new(format!(
                        "Invalid Git template locator, must be in the format of `{protocol}://url#revision`"
                    )));
                }
                "npm" | "pnpm" | "yarn" => {
                    if let Some(result) = NPM.captures(inner_value) {
                        return Ok(TemplateLocator::Npm {
                            package: result.name("package").unwrap().as_str().to_owned(),
                            version: Version::parse(result.name("version").unwrap().as_str())
                                .map_err(|error| ParseError::new(error.to_string()))?,
                        });
                    }

                    return Err(ParseError::new(format!(
                        "Invalid npm template locator, must be in the format of `{protocol}://package#version`"
                    )));
                }
                "file" => {
                    return Ok(TemplateLocator::File {
                        path: FilePath::from_str(inner_value)?,
                    });
                }
                "glob" => {
                    return Ok(TemplateLocator::Glob {
                        glob: GlobPath::from_str(inner_value)?,
                    });
                }
                other => {
                    return Err(ParseError::new(format!(
                        "Unknown template locator prefix `{other}`"
                    )));
                }
            };
        }

        Ok(if is_glob_like(value) {
            TemplateLocator::Glob {
                glob: GlobPath::from_str(value)?,
            }
        } else {
            TemplateLocator::File {
                path: FilePath::from_str(value)?,
            }
        })
    }
}

impl TryFrom<String> for TemplateLocator {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl From<TemplateLocator> for String {
    fn from(value: TemplateLocator) -> String {
        value.to_string()
    }
}
