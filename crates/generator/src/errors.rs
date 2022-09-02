use moon_error::MoonError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("A template with the name <id>{0}</id> already exists at <path>{1}</path>.")]
    ExistingTemplate(String, PathBuf),

    #[error("No template with the name <id>{0}</id> could be found at any of the configured template paths.")]
    MissingTemplate(String),

    #[error(transparent)]
    Moon(#[from] MoonError),
}
