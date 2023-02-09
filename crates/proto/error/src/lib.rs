use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtoError {
    #[error("Failed to download tool from {0}: {1}")]
    DownloadFailed(String, String),

    #[error("Unable to find an executable binary for {0}, expected file {1} does not exist.")]
    ExecuteMissingBin(String, PathBuf),

    #[error("File system failure for {0}: {1}")]
    Fs(PathBuf, String),

    #[error("HTTP failure for {0}: {1}")]
    Http(String, String),

    #[error("Unable to install {0}, download file is missing.")]
    InstallMissingDownload(String),

    #[error("Internet connection required, unable to download and install tools.")]
    InternetConnectionRequired,

    #[error("Invalid configuration for {0}: {1}")]
    InvalidConfig(PathBuf, String),

    #[error("JSON failure for {0}: {1}")]
    Json(PathBuf, String),

    #[error("{0}")]
    Message(String),

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("{0} has not been configured or installed, unable to proceed.")]
    MissingTool(String),

    #[error("Failed shim: {0}")]
    Shim(String),

    #[error("TOML failure for {0}: {1}")]
    Toml(PathBuf, String),

    #[error("Unable to install {0}, unsupported architecture {1}.")]
    UnsupportedArchitecture(String, String),

    #[error("Unable to install {0}, unsupported platform {1}.")]
    UnsupportedPlatform(String, String),

    #[error("Tool {0} is unknown or unsupported.")]
    UnsupportedTool(String),

    #[error("Checksum has failed for {0}, which was verified using {1}.")]
    VerifyInvalidChecksum(PathBuf, PathBuf),

    #[error("Version alias \"{0}\" could not be found in the manifest.")]
    VersionUnknownAlias(String),

    #[error("Failed to parse version {0}. {1}")]
    VersionParseFailed(String, String),

    #[error("Failed to resolve a semantic version for {0}.")]
    VersionResolveFailed(String),

    #[error("Failed zip archive. {0}")]
    Zip(String),
}
