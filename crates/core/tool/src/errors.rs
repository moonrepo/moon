use moon_error::MoonError;
use proto_error::ProtoError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Unable to find a binary for <symbol>{0}</symbol>. Have you installed the corresponding dependency?")]
    MissingBinary(String),

    #[error("{0} has not been configured or installed, unable to proceed.")]
    UnknownTool(String),

    #[error("{0}")]
    RequiresPlugin(String),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Proto(#[from] ProtoError),
}
