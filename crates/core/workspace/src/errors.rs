use moon_constants as constants;
use moon_error::MoonError;
use moon_vcs::VcsError;
use moonbase::MoonbaseError;
use proto::ProtoError;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error(
        "Unable to determine workspace root. Please create a {} configuration folder.",
        constants::CONFIG_DIRNAME.style(Style::File)
    )]
    MissingConfigDir,

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Unable to determine your current working directory.")]
    MissingWorkingDir,

    #[error(
        "Unable to locate {}/{} configuration file.",
        constants::CONFIG_DIRNAME.style(Style::File),
        constants::CONFIG_WORKSPACE_FILENAME.style(Style::File)
    )]
    MissingWorkspaceConfigFile,

    #[error(
        "Failed to validate {}/{} configuration file.\n\n{0}",
        constants::CONFIG_DIRNAME.style(Style::File),
        constants::CONFIG_TOOLCHAIN_FILENAME.style(Style::File)
    )]
    InvalidToolchainConfigFile(String),

    #[error(
        "Failed to validate {}/{} configuration file.\n\n{0}",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidWorkspaceConfigFile(String),

    #[error("Failed to validate {} configuration file.\n\n{1}", .0.style(Style::Path))]
    InvalidTasksConfigFile(PathBuf, String),

    #[error("Invalid moon version, unable to proceed. Found {0}, expected {1}.")]
    InvalidMoonVersion(String, String),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Moonbase(#[from] MoonbaseError),

    #[error(transparent)]
    Proto(#[from] ProtoError),

    #[error(transparent)]
    Vcs(#[from] VcsError),
}
