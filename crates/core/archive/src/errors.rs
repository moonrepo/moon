use moon_error::MoonError;
use starbase_utils::fs::FsError;
use starbase_utils::glob::GlobError;
use thiserror::Error;
use zip::result::ZipError;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error(transparent)]
    Fs(#[from] FsError),

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Zip(#[from] ZipError),
}
