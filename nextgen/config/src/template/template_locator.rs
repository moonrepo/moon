use crate::portable_path::{FilePath, PortablePath};
use schematic::ValidateError;
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
        revision: Option<String>,
    },
    Npm {
        package: String,
    },
}

impl fmt::Display for TemplateLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateLocator::File { path } => write!(f, "{path}"),
            TemplateLocator::Git {
                remote_url,
                revision,
            } => write!(
                f,
                "git:{remote_url}{}",
                match revision {
                    Some(rev) => format!("#{rev}"),
                    None => "".into(),
                }
            ),
            TemplateLocator::Npm { package } => write!(f, "npm:{package}"),
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
                "git" => {
                    let (remote_url, revision) = if let Some(rev_index) = inner_value.find('#') {
                        (
                            inner_value[0..rev_index].to_owned(),
                            Some(inner_value[rev_index + 1..].to_owned()),
                        )
                    } else {
                        (inner_value, None)
                    };

                    return Ok(TemplateLocator::Git {
                        remote_url,
                        revision,
                    });
                }
                "npm" => {
                    return Ok(TemplateLocator::Npm {
                        package: inner_value,
                    })
                }
                other => {
                    return Err(ValidateError::new(format!(
                        "Unknown template locator prefix {other}:."
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
