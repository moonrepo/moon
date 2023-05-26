use miette::Diagnostic;
use moon_common::consts;
use moon_config2::ConfigError;
use moon_error::MoonError;
use moon_vcs::VcsError;
use moonbase::MoonbaseError;
use proto::ProtoError;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum WorkspaceError {
    #[error(
        "Unable to determine workspace root. Please create a {} configuration folder.",
        consts::CONFIG_DIRNAME.style(Style::File)
    )]
    MissingConfigDir,

    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Unable to determine your current working directory.")]
    MissingWorkingDir,

    #[error(
        "Unable to locate {}/{} configuration file.",
        consts::CONFIG_DIRNAME.style(Style::File),
        consts::CONFIG_WORKSPACE_FILENAME.style(Style::File)
    )]
    MissingWorkspaceConfigFile,

    #[error(
        "Failed to validate {}/{} configuration file.\n\n{0}",
        consts::CONFIG_DIRNAME.style(Style::File),
        consts::CONFIG_TOOLCHAIN_FILENAME.style(Style::File)
    )]
    InvalidToolchainConfigFile(String),

    #[error(
        "Failed to validate {}/{} configuration file.\n\n{0}",
        consts::CONFIG_DIRNAME,
        consts::CONFIG_WORKSPACE_FILENAME
    )]
    InvalidWorkspaceConfigFile(String),

    #[error("Failed to validate {} configuration file.\n\n{1}", .0.style(Style::Path))]
    InvalidTasksConfigFile(PathBuf, String),

    #[error("Invalid moon version, unable to proceed. Found {0}, expected {1}.")]
    InvalidMoonVersion(String, String),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moonbase(#[from] MoonbaseError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Proto(#[from] ProtoError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Vcs(#[from] VcsError),
}
