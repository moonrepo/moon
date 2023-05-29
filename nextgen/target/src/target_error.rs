use miette::Diagnostic;
use moon_common::{IdError, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TargetError {
    #[diagnostic(code(target::invalid_format))]
    #[error(
        "Invalid target {}, must be in the format of \"scope:task\", with acceptable identifier characters.", .0.style(Style::Label)
    )]
    InvalidFormat(String),

    #[diagnostic(code(target::run_scope::no_deps))]
    #[error("Dependencies scope (^:) is not supported in run contexts.")]
    NoDepsInRunContext,

    #[diagnostic(code(target::run_scope::no_self))]
    #[error("Self scope (~:) is not supported in run contexts.")]
    NoSelfInRunContext,

    #[diagnostic(code(target::missing_segments))]
    #[error("Target \":\" encountered. Wildcard scope and task not supported.")]
    TooWild,

    #[diagnostic(transparent)]
    #[error(transparent)]
    IdError(#[from] IdError),
}
