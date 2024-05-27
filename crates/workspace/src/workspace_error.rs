use miette::Diagnostic;
use moon_common::consts;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum WorkspaceError {
    #[diagnostic(code(workspace::unknown_root))]
    #[error(
        "Unable to determine workspace root. Please create a {} configuration folder.",
        consts::CONFIG_DIRNAME.style(Style::File)
    )]
    MissingConfigDir,

    #[diagnostic(code(workspace::missing_config))]
    #[error(
        "Unable to locate {}/{} configuration file.",
        consts::CONFIG_DIRNAME.style(Style::File),
        consts::CONFIG_WORKSPACE_FILENAME.style(Style::File)
    )]
    MissingWorkspaceConfigFile,

    #[diagnostic(code(workspace::invalid_version))]
    #[error(
        "Invalid moon version, unable to proceed. Found {}, expected {}.",
        .actual.style(Style::Hash),
        .expected.style(Style::Hash)
    )]
    InvalidMoonVersion { actual: String, expected: String },
}
