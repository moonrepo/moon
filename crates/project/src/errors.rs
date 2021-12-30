use moon_config::{constants, ValidationErrors};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("A dependency cycle has been detected between projects.")]
    DependencyCycleDetected,

    #[error(
        "Failed to validate <path>{0}/{}</path> configuration file.",
        constants::CONFIG_PROJECT_FILENAME
    )]
    InvalidConfigFile(String, ValidationErrors),

    #[error("Failed to parse and open <path>{0}/package.json</path>: {1}")]
    InvalidPackageJson(String, String),

    #[error("No project exists at path <path>{0}</path>.")]
    MissingFilePath(String),

    #[error("No project has been configured with the ID <symbol>{0}</symbol>.")]
    UnconfiguredID(String),

    #[error("Task <symbol>{0}</symbol> has not been configured for project <symbol>{1}</symbol>.")]
    UnconfiguredTask(String, String),

    #[error("Unknown moon project error.")]
    Unknown,
}
