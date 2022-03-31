use moon_error::MoonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error(
        "Shashum check has failed for <file>{0}</file>, which was downloaded from <url>{1}</url>."
    )]
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

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error("HTTP: {0}")]
    HTTP(#[from] reqwest::Error),
}
