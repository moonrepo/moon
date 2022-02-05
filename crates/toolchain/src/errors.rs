use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("Unknown moon toolchain error.")]
    Unknown,

    #[error("Shashum check has failed for <file_path>{0}</file_path>, which was downloaded from <url>{1}</url>.")]
    InvalidShasum(
        String, // Download path
        String, // URL
    ),

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Unable to find a node module binary for <symbol>{0}</symbol>. Have you installed the corresponding package?")]
    MissingNodeModuleBin(String), // bin name

    #[error(
        "Unsupported architecture <symbol>{0}</symbol>. Unable to install <symbol>{1}</symbol>."
    )]
    UnsupportedArchitecture(
        String, // Arch
        String, // Tool name
    ),

    #[error("Unsupported platform <symbol>{0}</symbol>. Unable to install <symbol>{1}</symbol>.")]
    UnsupportedPlatform(
        String, // Platform
        String, // Tool name
    ),

    #[error("I/O: {0}")]
    IO(#[from] io::Error),

    #[error("HTTP: {0}")]
    HTTP(#[from] reqwest::Error),
}
