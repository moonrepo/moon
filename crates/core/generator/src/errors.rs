use moon_config::ConfigError;
use moon_constants as constants;
use moon_error::MoonError;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use tera::Error as TeraError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("A template with the name {} already exists at {}.", .0.style(Style::Id), .1.style(Style::Path))]
    ExistingTemplate(String, PathBuf),

    #[error("Failed to parse variable argument --{0}: {1}")]
    FailedToParseArgVar(String, String),

    #[error(
        "Failed to validate {} schema.\n\n{0}",
        constants::CONFIG_WORKSPACE_FILENAME.style(Style::File)
    )]
    InvalidConfigFile(String),

    #[error("No template with the name {} could not be found at any of the configured template paths.", .0.style(Style::Id))]
    MissingTemplate(String),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Tera(#[from] TeraError),
}
