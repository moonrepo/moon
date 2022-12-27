use moon_dep_graph::DepGraphError;
use moon_error::MoonError;
use moon_project::ProjectError;
use moon_tool::ToolError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error(transparent)]
    DepGraph(#[from] DepGraphError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Tool(#[from] ToolError),
}
