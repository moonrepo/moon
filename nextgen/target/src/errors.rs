use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TargetError {
    #[error(
        "Target {} requires fully-qualified scope and task identifiers, found {}.", .0.style(Style::Label), .1.style(Style::Label)
    )]
    IdOnly(String, String),

    #[error(
        "Invalid target {}, must be in the format of \"scope:task\".", .0.style(Style::Label)
    )]
    InvalidFormat(String),

    #[error("Target \":\" encountered. Wildcard scope and task not supported.")]
    TooWild,

    #[error(
        "All scope (:) is not supported in task deps, for target {}.", .0.style(Style::Label)
    )]
    NoAllInTaskDeps(String),

    #[error("Dependencies scope (^:) is not supported in run contexts.")]
    NoDepsInRunContext,

    #[error("Self scope (~:) is not supported in run contexts.")]
    NoSelfInRunContext,
}
