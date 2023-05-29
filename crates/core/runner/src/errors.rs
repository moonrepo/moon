use miette::Diagnostic;
use moon_error::MoonError;
use moon_process::ProcessError;
use moon_project::ProjectError;
use moon_target::TargetError;
use moon_task::TaskError;
use moon_tool::ToolError;
use moon_workspace::{VcsError, WorkspaceError};
use starbase_styles::{Style, Stylize};
use starbase_utils::glob::GlobError;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum RunnerError {
    #[diagnostic(code(target_runner::missing_dep_hash))]
    #[error(
        "Encountered a missing hash for target {}, which is a dependency of {}.\nThis either means the dependency hasn't ran, has failed, or there's a misconfiguration.\n\nTry disabling the target's cache, or marking it as local.",
        .0.style(Style::Label),
        .1.style(Style::Label),
    )]
    MissingDependencyHash(String, String),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Glob(#[from] GlobError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] ProcessError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Project(#[from] ProjectError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Target(#[from] TargetError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Task(#[from] TaskError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Tool(#[from] ToolError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Vcs(#[from] VcsError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
}
