use moon_error::MoonError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("No template with the name <id>{0}</id> could be found at any of the defined template paths.")]
    MissingTemplate(String),

    #[error("No template paths have been configured.")]
    NoTemplatePaths,

    #[error("A template with the name <id>{0}</id> already exists at <path>{1}</path>.")]
    TemplateAlreadyExists(String, PathBuf),

    #[error(transparent)]
    Moon(#[from] MoonError),
}
