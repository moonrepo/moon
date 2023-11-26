use miette::Diagnostic;
use moon_platform_runtime::Runtime;
use starbase_styles::{color, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ToolError {
    #[diagnostic(code(tool::missing_binary))]
    #[error("Unable to find a {0} for {}. Have you installed the corresponding dependency?", .1.style(Style::Symbol))]
    MissingBinary(String, String),

    #[diagnostic(code(tool::unknown))]
    #[error("{0} has not been configured or installed, unable to proceed.")]
    UnknownTool(String),

    #[diagnostic(code(tool::unsupported_platform))]
    #[error("Platform {0} has not been enabled or configured. Enable it with {}.", color::shell(format!("moon init {}", .0)))]
    UnsupportedPlatform(String),

    #[diagnostic(code(tool::unsupported_runtime))]
    #[error("Unsupported toolchain runtime {0}.")]
    UnsupportedRuntime(Runtime),

    #[diagnostic(code(tool::missing_plugin))]
    #[error("This functionality requires a plugin. Install it with {}.", .0.style(Style::Shell))]
    RequiresPlugin(String),
}
