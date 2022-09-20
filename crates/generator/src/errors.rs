use moon_config::ConfigError;
use moon_constants as constants;
use moon_error::MoonError;
use std::path::PathBuf;
use tera::Error as TeraError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("A template with the name <id>{0}</id> already exists at <path>{1}</path>.")]
    ExistingTemplate(String, PathBuf),

    #[error("Failed to parse variable argument --{0}: {1}")]
    FailedToParseArgVar(String, String),

    #[error(
        "Failed to validate <file>{}</file> schema.\n\n{0}",
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidConfigFile(String),

    #[error("No template with the name <id>{0}</id> could not be found at any of the configured template paths.")]
    MissingTemplate(String),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Tera(#[from] TeraError),
}
