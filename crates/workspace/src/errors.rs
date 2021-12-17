use monolith_config::{constants, ValidationErrors};
use monolith_project::ProjectError;
use monolith_toolchain::ToolchainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error(
        "Unable to determine workspace root. Please create a `{}` configuration folder.",
        constants::CONFIG_DIRNAME
    )]
    MissingConfigDir,

    #[error(
        "Unable to locate a root `package.json`. Please create one alongside the `{}` configuration folder.",
        constants::CONFIG_DIRNAME
    )]
    MissingPackageJson,

    #[error(
        "Unable to locate `{}/{}` configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    MissingWorkspaceConfigFile,

    #[error(
        "Unable to locate `{}/{}` configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_PROJECT_FILENAME
    )]
    MissingGlobalProjectConfigFile,

    #[error(
        "Failed to validate `{}/{}` configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidWorkspaceConfigFile(ValidationErrors),

    #[error(
        "Failed to validate `{}/{}` configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_PROJECT_FILENAME
    )]
    InvalidGlobalProjectConfigFile(ValidationErrors),

    #[error("Unknown monolith workspace error.")]
    Unknown,

    #[error("Project error.")]
    Project(#[from] ProjectError),

    #[error("Toolchain error.")]
    Toolchain(#[from] ToolchainError),
}
