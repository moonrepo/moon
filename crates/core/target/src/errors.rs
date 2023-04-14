use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TargetError {
    #[error(
        "Target {} requires literal project and task identifiers, found a scope.", .0.style(Style::Label)
    )]
    IdOnly(String),

    #[error(
        "Invalid target {}, must be in the format of \"project_id:task_id\".", .0.style(Style::Label)
    )]
    InvalidFormat(String),

    #[error("Target \":\" encountered. Wildcard project and task not supported.")]
    TooWild,

    #[error(
        "All projects scope (:) is not supported in task deps, for target {}.", .0.style(Style::Label)
    )]
    NoProjectAllInTaskDeps(String),

    #[error("Project dependencies scope (^:) is not supported in run contexts.")]
    NoProjectDepsInRunContext,

    #[error("Project self scope (~:) is not supported in run contexts.")]
    NoProjectSelfInRunContext,
}
