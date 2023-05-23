use moon_common::{Diagnostic, Error, IdError, Style, Stylize};

#[derive(Error, Debug)]
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

    #[error(transparent)]
    IdError(#[from] IdError),
}

impl Diagnostic for TargetError {}
