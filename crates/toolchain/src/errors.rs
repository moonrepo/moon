use moon_error::MoonError;
use moon_lang::LangError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("Internet connection required, unable to download and install tools.")]
    InternetConnectionRequired,

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Unable to find a node module binary for <symbol>{0}</symbol>. Have you installed the corresponding package?")]
    MissingNodeModuleBin(String), // bin name

    #[error(transparent)]
    Lang(#[from] LangError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error("HTTP: {0}")]
    HTTP(#[from] reqwest::Error),
}
