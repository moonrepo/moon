use moon_error::MoonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActionError {
    #[error(transparent)]
    Moon(#[from] MoonError),
}
