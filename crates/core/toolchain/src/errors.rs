use moon_archive::ArchiveError;
use moon_error::MoonError;
use moon_lang::LangError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("Failed to download tool from <url>{0}</url> <muted>({1})</muted>")]
    DownloadFailed(String, String),

    #[error("Internet connection required, unable to download and install tools.")]
    InternetConnectionRequired,

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Unable to find a node module binary for <symbol>{0}</symbol>. Have you installed the corresponding package?")]
    MissingNodeModuleBin(String), // bin name

    #[error("{0} has not been configured or installed, unable to proceed.")]
    MissingTool(String),

    #[error("This functionality requires workspace tools. Install it with <shell>yarn plugin import workspace-tools</shell>.")]
    RequiresYarnWorkspacesPlugin,

    #[error(transparent)]
    Archive(#[from] ArchiveError),

    #[error(transparent)]
    Lang(#[from] LangError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error("HTTP: {0}")]
    HTTP(#[from] reqwest::Error),
}
