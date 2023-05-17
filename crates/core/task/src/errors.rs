use moon_config::ConfigError;
use moon_error::MoonError;
use moon_target::TargetError;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Failed to parse env file {}: {1}", .0.style(Style::Path))]
    InvalidEnvFile(PathBuf, String),

    #[error(
        "Task outputs must be project relative and cannot be absolute. Found {} in {}.", .0.style(Style::File), .1.style(Style::Label)
    )]
    NoAbsoluteOutput(String, String),

    #[error(
        "Task outputs must be project relative and cannot traverse upwards. Found {} in {}.", .0.style(Style::File), .1.style(Style::Label)
    )]
    NoParentOutput(String, String),

    #[error("Target {} defines the output {}, but this output does not exist after being ran.", .0.style(Style::Label), .1.style(Style::File))]
    MissingOutput(String, String),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Target(#[from] TargetError),
}
