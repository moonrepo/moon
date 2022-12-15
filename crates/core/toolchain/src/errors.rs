use moon_error::MoonError;
use moon_platform::Runtime;
use proto_core::ProtoError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[error("Unable to find a node module binary for <symbol>{0}</symbol>. Have you installed the corresponding package?")]
    MissingNodeModuleBin(String), // bin name

    #[error("{0} has not been configured or installed, unable to proceed.")]
    MissingTool(String),

    #[error("Unsupported toolchain runtime {0}.")]
    UnsupportedRuntime(Runtime),

    #[error("This functionality requires workspace tools. Install it with <shell>yarn plugin import workspace-tools</shell>.")]
    RequiresYarnWorkspacesPlugin,

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Proto(#[from] ProtoError),
}
