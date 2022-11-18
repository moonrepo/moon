use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProbeError {
    #[error("Failed to download tool from {0}. {1}")]
    DownloadFailed(String, String),

    #[error("File system failure for {0}. {1}")]
    FileSystem(PathBuf, String),

    #[error("HTTP failure for {0}. {1}")]
    Http(String, String),

    #[error("Internet connection required, unable to download and install tools.")]
    InternetConnectionRequired,

    #[error("Checksum has failed for {0}, which was verified using {1}.")]
    InvalidChecksum(PathBuf, PathBuf),

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("{0} has not been configured or installed, unable to proceed.")]
    MissingTool(String),

    #[error("Unable to install {0}, unsupported architecture {1}.")]
    UnsupportedArchitecture(String, String),

    #[error("Unable to install {0}, unsupported platform {1}.")]
    UnsupportedPlatform(String, String),

    #[error("Failed to parse version {0}. {1}")]
    VersionParseFailed(String, String),

    #[error("Failed to resolve a semantic version for {0}.")]
    VersionResolveFailed(String),
}
