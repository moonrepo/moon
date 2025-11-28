#![allow(dead_code)]

use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum AppError {
    #[diagnostic(code(app::ci::no_shallow))]
    #[error(
        "CI requires a full VCS history or a base revision to operate correctly. Please avoid shallow checkouts."
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
        "Unable to determine workspace root. Please create a {} or {} configuration folder.",
        ".moon".style(Style::File),
        ".config/moon".style(Style::File),
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

    #[diagnostic(code(app::upgrade::requires_internet))]
    #[error("Upgrading moon requires an internet connection!")]
    UpgradeRequiresInternet,

    #[diagnostic(
        code(app::plugin::locator_required),
        url = "https://moonrepo.dev/docs/guides/wasm-plugins#configuring-plugin-locations"
    )]
    #[error("A plugin locator string is required for non-built-in plugins.")]
    PluginLocatorRequired,

    #[diagnostic(code(app::project_id_required))]
    #[error("A project identifier is required.")]
    ProjectIdRequired,

    #[diagnostic(code(app::template_id_required))]
    #[error("A template identifier is required.")]
    TemplateIdRequired,

    #[diagnostic(code(app::invalid_version))]
    #[error(
        "Invalid moon version, unable to proceed. Found {}, expected {}.",
        .actual.style(Style::Hash),
        .expected.style(Style::Hash)
    )]
    InvalidMoonVersion { actual: String, expected: String },

    #[diagnostic(code(app::tty::required_id))]
    #[error(
        "An identifier is required and must be explicitly provided as a positional argument in non-TTY environments."
    )]
    RequiredIdNonTTY,
}
