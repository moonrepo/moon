use monolith_config::{constants, ValidationErrors};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("No project exists at path `{0}`.")]
    DoesNotExist(String),

    #[error(
        "Failed to validate `{0}/{}` configuration file.",
        constants::CONFIG_PROJECT_FILENAME
    )]
    InvalidConfigFile(String, ValidationErrors),

    #[error("Failed to parse and open `{0}/package.json`: {1}")]
    InvalidPackageJson(String, String),

    #[error("No project has been configured with the ID `{0}`.")]
    UnconfiguredID(String),

    #[error("Unknown monolith project error.")]
    Unknown,
}
