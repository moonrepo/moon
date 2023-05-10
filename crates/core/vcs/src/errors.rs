use ignore::Error as IgnoreError;
use moon_error::MoonError;
use moon_process::ProcessError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VcsError {
    #[error("Failed to parse git remote URL. {0}")]
    FailedToParseGitRemote(String),

    #[error(transparent)]
    Ignore(#[from] IgnoreError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Process(#[from] ProcessError),
}
