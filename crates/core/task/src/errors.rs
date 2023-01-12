use moon_error::MoonError;
use moon_target::TargetError;
use moon_utils::glob::GlobError;
use moon_utils::process::ArgsParseError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Failed to parse env file <path>{0}</path>: {1}")]
    InvalidEnvFile(PathBuf, String),

    #[error(
        "Task outputs do not support file globs. Found <file>{0}</file> in <target>{1}</target>."
    )]
    NoOutputGlob(String, String),

    #[error(
        "Task outputs must be project relative and cannot be absolute. Found <file>{0}</file> in <target>{1}</target>."
    )]
    NoAbsoluteOutput(String, String),

    #[error(
        "Task outputs must be project relative and cannot traverse upwards. Found <file>{0}</file> in <target>{1}</target>."
    )]
    NoParentOutput(String, String),

    #[error("Target <target>{0}</target> defines the output <file>{1}</file>, but this output does not exist after being ran.")]
    MissingOutput(String, String),

    #[error(transparent)]
    ArgsParse(#[from] ArgsParseError),

    #[error(transparent)]
    FileGroup(#[from] FileGroupError),

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Target(#[from] TargetError),
}

#[derive(Error, Debug)]
pub enum FileGroupError {
    #[error("No globs defined in file group <id>{0}</id>.")]
    NoGlobs(String), // file group

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Moon(#[from] MoonError),
}
