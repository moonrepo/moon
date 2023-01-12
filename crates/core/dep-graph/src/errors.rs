use moon_error::MoonError;
use moon_project::ProjectError;
use moon_target::TargetError;
use moon_task::TaskError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DepGraphError {
    #[error("A dependency cycle has been detected for <file>{0}</file>.")]
    CycleDetected(String),

    #[error("Unknown node {0} found in dependency graph. How did this get here?")]
    UnknownNode(usize),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Task(#[from] TaskError),

    #[error(transparent)]
    Target(#[from] TargetError),
}
