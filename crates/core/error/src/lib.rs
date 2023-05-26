use miette::Diagnostic;
use regex::Error as RegexError;
use serde_json::Error as JsonError;
use serde_yaml::Error as YamlError;
use starbase_styles::{Style, Stylize};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::path::PathBuf;
use thiserror::Error;
use wax::GlobError;

// The native Rust IO error messages are not very intuitive as they do not include the
// file path that triggered the error. This file attemps to mitigate this by mapping
// over IO errors and including additional information.

#[derive(Error, Debug, Diagnostic)]
pub enum MoonError {
    #[error("{0}")]
    Generic(String),

    #[error("File system failure for {}: {1}", .0.style(Style::Path))]
    FileSystem(PathBuf, #[source] IoError),

    #[error("Glob failure for {}: {1}", .0.style(Style::File))]
    Glob(String, #[source] GlobError<'static>),

    #[error("Failed to create a hard link from {} to {}.", .0.style(Style::Path), .1.style(Style::Path))]
    HardLink(PathBuf, PathBuf),

    #[error("Failed to parse {}: {1}", .0.style(Style::Path))]
    Json(PathBuf, #[source] JsonError),

    #[error("Network failure: {0}")]
    Network(#[source] IoError),

    #[error("Network failure for {}: {1}", .0.style(Style::Path))]
    NetworkWithHandle(PathBuf, #[source] IoError),

    #[error("Path {} contains invalid UTF-8 characters.", .0.style(Style::Path))]
    PathInvalidUTF8(PathBuf),

    #[error("Process failure for {}: {1}", .0.style(Style::Shell))]
    Process(String, #[source] IoError),

    #[error("Process {} failed with a {} exit code.", .0.style(Style::Shell), .1.style(Style::Symbol))]
    ProcessNonZero(String, i32),

    #[error("Process {} failed with a {} exit code.\n{}", .0.style(Style::Shell), .1.style(Style::Symbol), .2.style(Style::MutedLight))]
    ProcessNonZeroWithOutput(String, i32, String),

    #[error("Platform {0} is not supported. Has it been configured or enabled?")]
    UnsupportedPlatform(String),

    #[error("Failed to parse {}: {1}", .0.style(Style::Path))]
    Yaml(PathBuf, #[source] YamlError),

    #[error(transparent)]
    Io(#[from] IoError),

    #[error(transparent)]
    Regex(#[from] RegexError),

    #[error("{0}")]
    Unknown(#[source] IoError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    StarFs(#[from] starbase_utils::fs::FsError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    StarGlob(#[from] starbase_utils::glob::GlobError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    StarJson(#[from] starbase_utils::json::JsonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    StarToml(#[from] starbase_utils::toml::TomlError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    StarYaml(#[from] starbase_utils::yaml::YamlError),
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

pub fn map_yaml_to_error(error: YamlError, path: PathBuf) -> MoonError {
    MoonError::Yaml(path, error)
}
