use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("Unknown moon toolchain error.")]
    Unknown,

    // TODO
    #[error("Command `{0}` failed to run.")]
    FailedCommandExec(
        String, // Command line
    ),

    #[error("Shashum check has failed for {0}, which was downloaded from {1}.")]
    InvalidShasum(
        String, // Download path
        String, // URL
    ),

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Unsupported architecture `{0}`. Unable to install {1}.")]
    UnsupportedArchitecture(
        String, // Arch
        String, // Tool name
    ),

    #[error("Unsupported platform `{0}`. Unable to install {1}.")]
    UnsupportedPlatform(
        String, // Platform
        String, // Tool name
    ),

    #[error("I/O")]
    IO(#[from] io::Error),

    #[error("HTTP")]
    HTTP(#[from] reqwest::Error),
}
