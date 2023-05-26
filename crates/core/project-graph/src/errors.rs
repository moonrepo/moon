use miette::Diagnostic;
use moon_common::IdError;
use moon_enforcer::EnforcerError;
use moon_error::MoonError;
use moon_file_group::FileGroupError;
use moon_project::ProjectError;
use moon_target::TargetError;
use moon_task::TaskError;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectGraphError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Enforcer(#[from] EnforcerError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Id(#[from] IdError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Project(#[from] ProjectError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Target(#[from] TargetError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Task(#[from] TaskError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Token(#[from] TokenError),
}

#[derive(Error, Debug, Diagnostic)]
pub enum TokenError {
    #[error(
        "Token {} received an invalid type for index \"{1}\", must be a number.", .0.style(Style::Symbol)
    )]
    InvalidIndexType(String, String), // token, index

    #[error("Input index {1} doesn't exist for token {}.", .0.style(Style::Symbol))]
    InvalidInIndex(String, u8), // token, index

    #[error("Output index {1} doesn't exist for token {}.", .0.style(Style::Symbol))]
    InvalidOutIndex(String, u8), // token, index

    #[error("Output token {} may not reference outputs using token functions.", .0.style(Style::Symbol))]
    InvalidOutNoTokenFunctions(String),

    #[error("Token {} cannot be used within {}.", .0.style(Style::Symbol), .0.style(Style::Symbol))]
    InvalidTokenContext(String, String), // token, context

    #[error("Unknown file group {} used in token {}.", .1.style(Style::Id), .0.style(Style::Symbol))]
    UnknownFileGroup(String, String), // token, file group

    #[error("Unknown token function {}.", .0.style(Style::Symbol))]
    UnknownTokenFunc(String), // token

    #[diagnostic(transparent)]
    #[error(transparent)]
    FileGroup(#[from] FileGroupError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Target(#[from] TargetError),
}
