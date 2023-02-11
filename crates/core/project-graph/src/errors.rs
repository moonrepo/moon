use moon_error::MoonError;
use moon_project::ProjectError;
use moon_target::TargetError;
use moon_task::{FileGroupError, TaskError};
use moon_utils::glob::GlobError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectGraphError {
    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Target(#[from] TargetError),

    #[error(transparent)]
    Task(#[from] TaskError),

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

    #[error("Output token <symbol>{0}</symbol> may not reference outputs using token function.")]
    InvalidOutNoTokenFunctions(String),

    #[error("Token <symbol>{0}</symbol> cannot be used within <id>{1}</id>.")]
    InvalidTokenContext(String, String), // token, context

    #[error("Unknown file group <id>{1}</id> used in token <symbol>{0}</symbol>.")]
    UnknownFileGroup(String, String), // token, file group

    #[error("Unknown token function <symbol>{0}</symbol>.")]
    UnknownTokenFunc(String), // token

    #[error(transparent)]
    FileGroup(#[from] FileGroupError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Target(#[from] TargetError),
}
