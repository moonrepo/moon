use ignore::Error as IgnoreError;
use moon_error::MoonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VcsError {
    #[error(transparent)]
    Ignore(#[from] IgnoreError),

    #[error(transparent)]
    Moon(#[from] MoonError),
}
