use moon_config::{constants, ValidationErrors};
use moon_error::MoonError;
use moon_project::ProjectError;
use moon_toolchain::ToolchainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("A dependency cycle has been detected for <path>{0}</path>.")]
    DepGraphCycleDetected(String),

    #[error("Unknown node {0} found in dependency graph. How did this get here?")]
    DepGraphUnknownNode(usize),

    #[error("Task runner failed to run: {0}")]
    TaskRunnerFailure(#[from] tokio::task::JoinError),

    #[error("Target <id>{0}</id> failed to run.")]
    TaskRunnerFailedTarget(String),

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
        "Failed to validate <path>{}/{}</path> configuration file.\n\n<muted>{0}</muted>",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidWorkspaceConfigFile(ValidationErrors),

    #[error(
        "Failed to validate <path>{}/{}</path> configuration file.\n\n<muted>{0}</muted>",
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
