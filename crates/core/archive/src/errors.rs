use starbase_utils::fs::FsError;
use starbase_utils::glob::GlobError;
use thiserror::Error;
use zip::result::ZipError;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Fs(#[from] FsError),

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Zip(#[from] ZipError),
}
