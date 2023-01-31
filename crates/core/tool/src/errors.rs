use moon_error::MoonError;
use moon_platform_runtime::Runtime;
use proto::ProtoError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Unable to find a binary for <symbol>{0}</symbol>. Have you installed the corresponding dependency?")]
    MissingBinary(String),

    #[error("{0} has not been configured or installed, unable to proceed.")]
    UnknownTool(String),

    #[error("Unsupported toolchain runtime {0}.")]
    UnsupportedRuntime(Runtime),

    #[error("This functionality requires a plugin. Install it with <shell>{0}</shell>.")]
    RequiresPlugin(String),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Proto(#[from] ProtoError),
}
