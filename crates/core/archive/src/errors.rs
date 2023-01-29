use moon_error::MoonError;
use moon_utils::glob::GlobError;
use thiserror::Error;
use zip::result::ZipError;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Zip(#[from] ZipError),
}
