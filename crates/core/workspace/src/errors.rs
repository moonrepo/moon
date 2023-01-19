use std::path::PathBuf;

use moon_constants as constants;
use moon_error::MoonError;
use moon_utils::glob::GlobError;
use moon_vcs::VcsError;
use moonbase::MoonbaseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error(
        "Unable to determine workspace root. Please create a <file>{}</file> configuration folder.",
        constants::CONFIG_DIRNAME
    )]
    MissingConfigDir,

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error(
        "Unable to locate <file>{}/{}</file> configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    MissingWorkspaceConfigFile,

    #[error(
        "Failed to validate <file>{}/{}</file> configuration file.\n\n{0}",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_TOOLCHAIN_FILENAME
    )]
    InvalidToolchainConfigFile(String),

    #[error(
        "Failed to validate <file>{}/{}</file> configuration file.\n\n{0}",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidWorkspaceConfigFile(String),

    #[error("Failed to validate <file>{0}</file> configuration file.\n\n{1}")]
    InvalidTasksConfigFile(PathBuf, String),

    #[error("Invalid moon version, unable to proceed. Found {0}, expected {1}.")]
    InvalidMoonVersion(String, String),

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Moonbase(#[from] MoonbaseError),

    #[error(transparent)]
    Vcs(#[from] VcsError),
}
