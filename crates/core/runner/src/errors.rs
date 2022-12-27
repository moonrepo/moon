use moon_error::MoonError;
use moon_project::ProjectError;
use moon_task::{TargetError, TaskError};
use moon_workspace::{VcsError, WorkspaceError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Target(#[from] TargetError),

    #[error(transparent)]
    Task(#[from] TaskError),

    #[error(transparent)]
    Vcs(#[from] VcsError),

    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
}
