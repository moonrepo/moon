use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Failed to create a directory.")]
    FailedToCreateDir,

    #[error("Failed to download tool.")]
    FailedToDownload,

    #[error("Failed to install tool.")]
    FailedToInstall,

    #[error("Failed to unload tool from toolchain.")]
    FailedToUnload,

    #[error("Unsupported architecture.")]
    UnsupportedArchitecture(String),

    #[error("Unsupported platform.")]
    UnsupportedPlatform(String),

    #[error("Unknown monolith toolchain error.")]
    Unknown,
}
