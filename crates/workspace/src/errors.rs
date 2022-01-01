use moon_config::{constants, ValidationErrors};
use moon_project::ProjectError;
use moon_toolchain::ToolchainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error(
        "Unable to determine workspace root. Please create a <path>{}</path> configuration folder.",
        constants::CONFIG_DIRNAME
    )]
    MissingConfigDir,

    #[error(
        "Unable to locate a root <path>package.json</path>. Please create one alongside the <path>{}</path> configuration folder.",
        constants::CONFIG_DIRNAME
    )]
    MissingPackageJson,

    #[error(
        "Unable to locate <path>{}/{}</path> configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    MissingWorkspaceConfigFile,

    #[error(
        "Unable to locate <path>{}/{}</path> configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_PROJECT_FILENAME
    )]
    MissingGlobalProjectConfigFile,

    #[error(
        "Failed to validate <path>{}/{}</path> configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidWorkspaceConfigFile(ValidationErrors),

    #[error(
        "Failed to validate <path>{}/{}</path> configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_PROJECT_FILENAME
    )]
    InvalidGlobalProjectConfigFile(ValidationErrors),

    #[error("Unknown moon workspace error.")]
    Unknown,

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Toolchain(#[from] ToolchainError),

    #[error(transparent)]
    Vcs(#[from] VcsError),
}

#[derive(Error, Debug)]
pub enum VcsError {
    #[error("I/O: {0}")]
    IO(#[from] std::io::Error),
}
