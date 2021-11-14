use monolith_config::{constants, ValidationErrors};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error(
        "Unable to determine workspace root. Please create a `{}` configuration folder.",
        constants::CONFIG_DIRNAME
    )]
    MissingConfigDir,

    #[error(
        "Unable to locate `{}/{}` configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    MissingWorkspaceConfigFile,

    #[error(
        "Failed to validate `{}/{}` configuration file.",
        constants::CONFIG_DIRNAME,
        constants::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidWorkspaceConfigFile(ValidationErrors),

    #[error("Unknown monolith workspace error.")]
    Unknown,
}
