use miette::Diagnostic;
use moon_dep_graph::DepGraphError;
use moon_error::MoonError;
use moon_project::ProjectError;
use moon_runner::RunnerError;
use moon_target::TargetError;
use moon_tool::ToolError;
use moon_workspace::WorkspaceError;
use starbase_utils::fs::FsError;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum PipelineError {
    #[error("{0}")]
    Aborted(String),

    #[error("An unknown action was encountered in the pipeline. Unable to proceed!")]
    UnknownActionNode,

    #[diagnostic(transparent)]
    #[error(transparent)]
    DepGraph(#[from] DepGraphError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] FsError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Project(#[from] ProjectError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Runner(#[from] RunnerError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Target(#[from] TargetError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Tool(#[from] ToolError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
}
