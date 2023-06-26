use miette::Diagnostic;
use moon_common::consts;
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
}
