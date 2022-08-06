use moon_error::MoonError;
use thiserror::Error;
use zip::result::ZipError;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Zip(#[from] ZipError),
}
