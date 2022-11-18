use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProbeError {
    #[error("Failed to download tool from <url>{0}</url> <muted>({1})</muted>")]
    DownloadFailed(String, String),

    #[error("Internet connection required, unable to download and install tools.")]
    InternetConnectionRequired,

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("{0} has not been configured or installed, unable to proceed.")]
    MissingTool(String),
    // #[error("HTTP: {0}")]
    // HTTP(#[from] reqwest::Error),
}
