#![allow(dead_code)]

use miette::Diagnostic;
use moon_common::{consts, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum AppError {
    #[diagnostic(code(app::workspace::invalid_root_env))]
    #[error(
        "Unable to determine workspace root. Failed to parse {} into a valid path.",
        "MOON_WORKSPACE_ROOT".style(Style::Symbol)
    )]
    InvalidWorkspaceRootEnvVar,

    #[diagnostic(code(app::missing_workspace))]
    #[error(
        "Unable to determine workspace root. Please create a {} configuration folder.",
        consts::CONFIG_DIRNAME.style(Style::File)
    )]
    MissingConfigDir,

    #[diagnostic(code(app::missing_config))]
    #[error(
        "Unable to locate {} configuration file.",
        .0.style(Style::File),
    )]
    MissingConfigFile(String),

    #[diagnostic(code(app::missing_home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(app::missing_working_dir))]
    #[error("Unable to determine your current working directory.")]
    MissingWorkingDir,
}
