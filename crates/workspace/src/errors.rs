use moon_config::{constants, ValidationErrors};
use moon_error::MoonError;
use moon_project::ProjectError;
use moon_toolchain::ToolchainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("A dependency cycle has been detected for <file>{0}</file>.")]
    DepGraphCycleDetected(String),

    #[error("Unknown node {0} found in dependency graph. How did this get here?")]
    DepGraphUnknownNode(usize),

    #[error("{0}")]
    ActionRunnerFailure(String),

    #[error(
        "Unable to determine workspace root. Please create a <file>{}</file> configuration folder.",
        constants::CONFIG_DIRNAME
    )]
    MissingConfigDir,

    #[error(
        "Unable to locate a root <file>package.json</file>. Please create one alongside the <file>{}</file> configuration folder.",
        constants::CONFIG_DIRNAME
    )]
    MissingPackageJson,

    #[error(
        "Unable to locate <file>{}/{}</file> configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    MissingWorkspaceConfigFile,

    #[error(
        "Failed to validate <file>{}/{}</file> configuration file.\n\n<muted>{0}</muted>",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidWorkspaceConfigFile(ValidationErrors),

    #[error(
        "Failed to validate <file>{}/{}</file> configuration file.\n\n<muted>{0}</muted>",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_PROJECT_FILENAME
    )]
    InvalidGlobalProjectConfigFile(ValidationErrors),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Toolchain(#[from] ToolchainError),
}
