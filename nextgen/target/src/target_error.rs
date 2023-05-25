use miette::Diagnostic;
use moon_common::{IdError, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TargetError {
    #[error(
        "Invalid target {}, must be in the format of \"scope:task\", with acceptable identifier characters.", .0.style(Style::Label)
    )]
    InvalidFormat(String),

    #[error(
        "All scope (:) is not supported in task deps, for target {}.", .0.style(Style::Label)
    )]
    NoAllInTaskDeps(String),

    #[error(
        "Tag scope (#tag:) is not supported in task deps, for target {}.", .0.style(Style::Label)
    )]
    NoTagInTaskDeps(String),

    #[error("Dependencies scope (^:) is not supported in run contexts.")]
    NoDepsInRunContext,

    #[error("Self scope (~:) is not supported in run contexts.")]
    NoSelfInRunContext,

    #[error("Target \":\" encountered. Wildcard scope and task not supported.")]
    TooWild,

    #[diagnostic(transparent)]
    #[error(transparent)]
    IdError(#[from] IdError),
}
