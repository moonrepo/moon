use moon_error::MoonError;
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

    #[error(
        "All projects scope (:) is not supported in task deps, for target <target>{0}</target>."
    )]
    NoProjectAllInTaskDeps(String),

    #[error("Project dependencies scope (^:) is not supported in run contexts.")]
    NoProjectDepsInRunContext,

    #[error("Project self scope (~:) is not supported in run contexts.")]
    NoProjectSelfInRunContext,
}
