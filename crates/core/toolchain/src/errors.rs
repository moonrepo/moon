use moon_error::MoonError;
use moon_lang::LangError;
use probe_core::ProbeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Unable to find a node module binary for <symbol>{0}</symbol>. Have you installed the corresponding package?")]
    MissingNodeModuleBin(String), // bin name

    #[error("{0} has not been configured or installed, unable to proceed.")]
    MissingTool(String),

    #[error("This functionality requires workspace tools. Install it with <shell>yarn plugin import workspace-tools</shell>.")]
    RequiresYarnWorkspacesPlugin,

    #[error(transparent)]
    Lang(#[from] LangError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Probe(#[from] ProbeError),
}
