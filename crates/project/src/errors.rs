use moon_config::{constants, ValidationErrors};
use moon_error::MoonError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("A dependency cycle has been detected between projects.")]
    DependencyCycleDetected,

    #[error(
        "Failed to validate <file>{0}/{}</file> configuration file.\n\n<muted>{0}</muted>",
        constants::CONFIG_PROJECT_FILENAME
    )]
    InvalidConfigFile(String, ValidationErrors),

    #[error("Failed to parse and open <file>{0}/package.json</file>: {1}")]
    InvalidPackageJson(String, String),

    #[error("Failed to parse and open <file>{0}/{1}</file>: {2}")]
    InvalidTsConfigJson(String, String, String),

    #[error("No project exists at path <file>{0}</file>.")]
    MissingProject(String),

    #[error(
        "Task outputs do not support file globs. Found <path>{0}</path> in <target>{1}</target>."
    )]
    NoOutputGlob(PathBuf, String),

    #[error("No project has been configured with the ID <id>{0}</id>.")]
    UnconfiguredID(String),

    #[error("Task <id>{0}</id> has not been configured for project <id>{1}</id>.")]
    UnconfiguredTask(String, String),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    GlobWalk(#[from] globwalk::GlobError),

    #[error(transparent)]
    GlobSet(#[from] globset::Error),

    #[error(transparent)]
    Target(#[from] TargetError),

    #[error(transparent)]
    Token(#[from] TokenError),
}

#[derive(Error, Debug)]
pub enum TargetError {
    #[error(
        "Target <target>{0}</target> requires literal project and task identifiers, found a scope."
    )]
    IdOnly(String),

    #[error(
        "Invalid target <target>{0}</target>, must be in the format of \"project_id:task_id\"."
    )]
    InvalidFormat(String),

    #[error("Target <target>:</target> encountered. Wildcard project and task not supported.")]
    TooWild,
}

#[derive(Error, Debug)]
pub enum TokenError {
    #[error(
        "Token <symbol>{0}</symbol> received an invalid type for index \"{1}\", must be a number."
    )]
    InvalidIndexType(String, String), // token, index

    #[error("Input index {1} doesn't exist for token <symbol>{0}</symbol>.")]
    InvalidInIndex(String, u8), // token, index

    #[error("Output index {1} doesn't exist for token <symbol>{0}</symbol>.")]
    InvalidOutIndex(String, u8), // token, index

    #[error("Token <symbol>{0}</symbol> cannot be used within <id>{1}</id>.")]
    InvalidTokenContext(String, String), // token, context

    #[error("No globs defined in file group <id>{0}</id>.")]
    NoGlobs(String), // file group

    #[error("Unknown file group <id>{1}</> used in token <symbol>{0}</symbol>.")]
    UnknownFileGroup(String, String), // token, file group

    #[error("Unknown token function <symbol>{0}</symbol>.")]
    UnknownTokenFunc(String), // token
}
