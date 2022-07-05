use moon_error::MoonError;
use moon_project::ProjectError;
use moon_toolchain::ToolchainError;
use moon_vcs::VcsError;
use moon_workspace::WorkspaceError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActionRunnerError {
    #[error("{0}")]
    Failure(String),

    #[error(transparent)]
    DepGraph(#[from] DepGraphError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Toolchain(#[from] ToolchainError),

    #[error(transparent)]
    Vcs(#[from] VcsError),

    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
}

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
}
