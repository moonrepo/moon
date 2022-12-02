use moon_dep_graph::DepGraphError;
use moon_error::MoonError;
use moon_project::ProjectError;
use moon_task::{TargetError, TaskError};
use moon_toolchain::ToolchainError;
use moon_vcs::VcsError;
use moon_workspace::WorkspaceError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("{0}")]
    Failure(String),

    #[error(transparent)]
    DepGraph(#[from] DepGraphError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Target(#[from] TargetError),

    #[error(transparent)]
    Task(#[from] TaskError),

    #[error(transparent)]
    Toolchain(#[from] ToolchainError),

    #[error(transparent)]
    Vcs(#[from] VcsError),

    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
}
