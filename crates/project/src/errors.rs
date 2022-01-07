use moon_config::{constants, ValidationErrors};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("Unknown moon project error.")]
    Unknown,

    #[error("A dependency cycle has been detected between projects.")]
    DependencyCycleDetected,

    #[error(
        "Failed to validate <path>{0}/{}</path> configuration file.\n\n<muted>{0}</muted>",
        constants::CONFIG_PROJECT_FILENAME
    )]
    InvalidConfigFile(String, ValidationErrors),

    #[error("Failed to parse and open <path>{0}/package.json</path>: {1}")]
    InvalidPackageJson(String, String),

    #[error("Invalid target <id>{0}</id>, must be in the format of \"project_id:task_id\".")]
    InvalidTargetFormat(String),

    #[error(
        "Invalid or missing file <file_path>{0}</file_path>, must be a valid UTF-8 file path."
    )]
    InvalidUtf8File(PathBuf),

    #[error("No file exists at path <file_path>{0}</file_path>.")]
    MissingFile(PathBuf),

    #[error("No project exists at path <path>{0}</path>.")]
    MissingProject(String),

    #[error("Task outputs do not support file globs. Found <path>{0}</path> in <id>{1}<id>.")]
    NoOutputGlob(String, String),

    #[error("No project has been configured with the ID <id>{0}</id>.")]
    UnconfiguredID(String),

    #[error("Task <id>{0}</id> has not been configured for project <id>{1}</id>.")]
    UnconfiguredTask(String, String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    GlobWalk(#[from] globwalk::GlobError),

    #[error(transparent)]
    GlobSet(#[from] globset::Error),

    #[error(transparent)]
    Token(#[from] TokenError),
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
