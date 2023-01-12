use thiserror::Error;

#[derive(Error, Debug)]
pub enum TargetError {
    #[error(
        "Target <target>{0}</target> requires literal project and task identifiers, found a scope."
    )]
    IdOnly(String),

    #[error(
        "Invalid target <target>{0}</target>, must be in the format of \"project_id:task_id\"."
    )]
    InvalidFormat(String),

    #[error("Target <target>:</target> encountered. Wildcard project and task not supported.")]
    TooWild,

    #[error(
        "All projects scope (:) is not supported in task deps, for target <target>{0}</target>."
    )]
    NoProjectAllInTaskDeps(String),

    #[error("Project dependencies scope (^:) is not supported in run contexts.")]
    NoProjectDepsInRunContext,

    #[error("Project self scope (~:) is not supported in run contexts.")]
    NoProjectSelfInRunContext,
}
