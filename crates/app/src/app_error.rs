#![allow(dead_code)]

use miette::Diagnostic;
use moon_common::{consts, Id, Style, Stylize};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("Exit with code")]
pub struct ExitCode(pub i32);

#[derive(Error, Debug, Diagnostic)]
pub enum AppError {
    #[diagnostic(code(app::ci::no_shallow))]
    #[error(
        "CI requires a full VCS history to operate correctly. Please avoid shallow checkouts."
    )]
    CiNoShallowHistory,

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

    #[diagnostic(code(app::missing_hash_manifest))]
    #[error(
        "Unable to find a hash manifest for {}!",
        .0.style(Style::Hash),
    )]
    MissingHashManifest(String),

    #[diagnostic(code(app::missing_home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(app::missing_working_dir))]
    #[error("Unable to determine your current working directory.")]
    MissingWorkingDir,

    #[diagnostic(code(app::extensions::unknown_id))]
    #[error(
        "The extension {} does not exist. Configure the {} setting in {} and try again.",
        .id.style(Style::Id),
        "extensions".style(Style::Property),
        ".moon/workspace.yml".style(Style::File),
    )]
    UnknownExtension { id: Id },
}
