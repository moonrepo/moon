use moon_error::MoonError;
use moon_project::ProjectError;
use moon_target::TargetError;
use moon_task::TaskError;
use moon_tool::ToolError;
use moon_workspace::{VcsError, WorkspaceError};
use starbase_styles::{Style, Stylize};
use starbase_utils::glob::GlobError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Encountered a missing hash for target {}, which is a dependency of {}. This either means the dependency hasn't ran, has failed, or there's a misconfiguration.", .0.style(Style::Label), .1.style(Style::Label))]
    MissingDependencyHash(String, String),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Glob(#[from] GlobError),

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
