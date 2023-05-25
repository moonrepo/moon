use miette::Diagnostic;
use moon_error::MoonError;
use moon_platform_runtime::Runtime;
use moon_process::ProcessError;
use proto::ProtoError;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ToolError {
    #[error("Unable to find a {0} for {}. Have you installed the corresponding dependency?", .1.style(Style::Symbol))]
    MissingBinary(String, String),

    #[error("{0} has not been configured or installed, unable to proceed.")]
    UnknownTool(String),

    #[error("Unsupported toolchain runtime {0}.")]
    UnsupportedRuntime(Runtime),

    #[error("This functionality requires a plugin. Install it with {}.", .0.style(Style::Shell))]
    RequiresPlugin(String),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] ProcessError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Proto(#[from] ProtoError),
}
