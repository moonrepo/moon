use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_error::MoonError;
use moon_task::{TargetError, TaskError};
use moon_utils::glob::GlobError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error(
        "Failed to validate <file>{0}/{}</file> configuration file.\n\n{1}",
        CONFIG_PROJECT_FILENAME
    )]
    InvalidConfigFile(String, String),

    #[error("No project exists at path <file>{0}</file>.")]
    MissingProject(String),

    #[error("No project has been configured with the ID <id>{0}</id>.")]
    UnconfiguredID(String),

    #[error("Task <id>{0}</id> has not been configured for project <id>{1}</id>.")]
    UnconfiguredTask(String, String),

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Target(#[from] TargetError),

    #[error(transparent)]
    Task(#[from] TaskError),
    // #[error(transparent)]
    // Token(#[from] TokenError),
}
