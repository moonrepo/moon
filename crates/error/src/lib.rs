use regex::Error as RegexError;
use serde_json::Error as JsonError;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::path::PathBuf;
use thiserror::Error;

// The native Rust IO error messages are not very intuitive as they do not include the
// file path that triggered the error. This file attemps to mitigate this by mapping
// over IO errors and including additional information.

#[derive(Error, Debug)]
pub enum MoonError {
    #[error("{0}")]
    Generic(String),

    #[error("File system failure for <path>{0}</path>: {1}")]
    FileSystem(PathBuf, #[source] IoError),

    #[error("Failed to create a hard link from <path>{0}</path> to <path>{1}</path>.")]
    HardLink(PathBuf, PathBuf),

    #[error("Failed to parse <path>{0}</path>: {1}")]
    Json(PathBuf, #[source] JsonError),

    #[error("Network failure: {0}")]
    Network(#[source] IoError),

    #[error("Network failure for <path>{0}</path>: {1}")]
    NetworkWithHandle(PathBuf, #[source] IoError),

    #[error("Path <path>{0}</path> contains invalid UTF-8 characters.")]
    PathInvalidUTF8(PathBuf),

    #[error("Process failure for <shell>{0}</shell>: {1}")]
    Process(String, #[source] IoError),

    #[error("Process <shell>{0}</shell> failed with a <symbol>{1}</symbol> exit code.")]
    ProcessNonZero(String, i32),

    #[error("Process <shell>{0}</shell> failed with a <symbol>{1}</symbol> exit code.\n<muted>{2}</muted>")]
    ProcessNonZeroWithOutput(String, i32, String),

    #[error(transparent)]
    Io(#[from] IoError),

    #[error(transparent)]
    Regex(#[from] RegexError),

    #[error("{0}")]
    Unknown(#[source] IoError),
}

pub fn map_io_to_fs_error(error: IoError, path: PathBuf) -> MoonError {
    match error.kind() {
        IoErrorKind::AlreadyExists
        // | IoErrorKind::Deadlock
        // | IoErrorKind::DirectoryNotEmpty
        // | IoErrorKind::ExecutableFileBusy
        // | IoErrorKind::FilesystemQuotaExceeded
        // | IoErrorKind::FilenameTooLong
        // | IoErrorKind::FileTooLarge
        | IoErrorKind::InvalidData
        // | IoErrorKind::IsADirectory
        // | IoErrorKind::NotADirectory
        | IoErrorKind::NotFound
        // | IoErrorKind::NotSeekable
        | IoErrorKind::Other
        | IoErrorKind::PermissionDenied
        // | IoErrorKind::ReadOnlyFilesystem
        // | IoErrorKind::StorageFull
        // | IoErrorKind::TooManyLinks
        // | IoErrorKind::Uncategorized
        | IoErrorKind::UnexpectedEof => MoonError::FileSystem(path, error),
        _ => MoonError::Network(error),
    }
}

pub fn map_io_to_net_error(error: IoError, handle: Option<PathBuf>) -> MoonError {
    match error.kind() {
        IoErrorKind::AddrInUse
        | IoErrorKind::AddrNotAvailable
        | IoErrorKind::BrokenPipe
        | IoErrorKind::ConnectionAborted
        | IoErrorKind::ConnectionRefused
        | IoErrorKind::ConnectionReset
        // | IoErrorKind::HostUnreachable
        // | IoErrorKind::NetworkDown
        // | IoErrorKind::NetworkUnreachable
        | IoErrorKind::NotConnected
        // | IoErrorKind::ResourceBusy
        // | IoErrorKind::StaleNetworkFileHandle
        | IoErrorKind::TimedOut
        | IoErrorKind::WriteZero => {
            if let Some(path) = handle {
                MoonError::NetworkWithHandle(path, error)
            } else {
                MoonError::Network(error)
            }
        },
        _ => MoonError::Network(error),
    }
}

pub fn map_io_to_process_error(error: IoError, bin: &str) -> MoonError {
    MoonError::Process(String::from(bin), error)
}

pub fn map_json_to_error(error: JsonError, path: PathBuf) -> MoonError {
    MoonError::Json(path, error)
}
