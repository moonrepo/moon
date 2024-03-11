use crate::portable_path::{FilePath, PortablePath};
use schematic::ValidateError;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged, try_from = "String", into = "String")]
pub enum TemplateLocator {
    File {
        path: FilePath,
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

impl fmt::Display for TemplateLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateLocator::File { path } => write!(f, "{path}"),
            TemplateLocator::Git {
                remote_url,
                revision,
            } => write!(f, "git:{remote_url}#{revision}"),
            TemplateLocator::Npm { package, version } => write!(f, "npm:{package}@{version}"),
        }
    }
}

impl schematic::Schematic for TemplateLocator {
    fn generate_schema() -> schematic::SchemaType {
        schematic::SchemaType::string()
    }
}

impl FromStr for TemplateLocator {
    type Err = ValidateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(index) = value.find(':') {
            let inner_value = value[index + 1..].to_owned();

            match &value[0..index] {
                "git" | "git+http" | "git+https" => {
                    let (remote_url, revision) = if let Some(inner_index) = inner_value.find('#') {
                        (
                            inner_value[0..inner_index].to_owned(),
                            inner_value[inner_index + 1..].to_owned(),
                        )
                    } else {
                        return Err(ValidateError::new(format!(
                            "Git template locator is missing a revision (commit, branch, etc)"
                        )));
                    };

                    return Ok(TemplateLocator::Git {
                        remote_url,
                        revision,
                    });
                }
                "npm" | "pnpm" | "yarn" => {
                    // Don't find leading @ when a scope is being used!
                    let (package, version) = if let Some(inner_index) = inner_value[1..].find('@') {
                        (
                            inner_value[0..inner_index].to_owned(),
                            Version::parse(&inner_value[inner_index + 1..])
                                .map_err(|error| ValidateError::new(error.to_string()))?,
                        )
                    } else {
                        return Err(ValidateError::new(format!(
                            "npm template locator is missing a semantic version"
                        )));
                    };

                    return Ok(TemplateLocator::Npm { package, version });
                }
                other => {
                    return Err(ValidateError::new(format!(
                        "Unknown template locator prefix `{other}:`"
                    )));
                }
            };
        }

        let path = FilePath::from_str(value)?;

        Ok(TemplateLocator::File { path })
    }
}

impl TryFrom<String> for TemplateLocator {
    type Error = ValidateError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl From<TemplateLocator> for String {
    fn from(value: TemplateLocator) -> String {
        value.to_string()
    }
}
