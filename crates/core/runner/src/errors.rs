use moon_error::MoonError;
use moon_project::ProjectError;
use moon_target::TargetError;
use moon_task::TaskError;
use moon_tool::ToolError;
use moon_utils::glob::GlobError;
use moon_workspace::{VcsError, WorkspaceError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Encountered an empty hash for target <target>{0}</target>, which is a dependency of <target>{1}</target>. This either means the dependency hasn't ran, has failed, or there's a misconfiguration.")]
    EmptyDependencyHash(String, String),

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Target(#[from] TargetError),

    #[error(transparent)]
    Task(#[from] TaskError),

    #[error(transparent)]
    Tool(#[from] ToolError),

    #[error(transparent)]
    Vcs(#[from] VcsError),

    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
}
