use ignore::Error as IgnoreError;
use miette::Diagnostic;
use moon_error::MoonError;
use moon_process::ProcessError;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum VcsError {
    #[error("Failed to parse git remote URL. {0}")]
    FailedToParseGitRemote(String),

    #[error(transparent)]
    Ignore(#[from] IgnoreError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] ProcessError),
}
