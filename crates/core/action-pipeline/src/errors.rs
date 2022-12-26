use moon_dep_graph::DepGraphError;
use moon_error::MoonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error(transparent)]
    DepGraph(#[from] DepGraphError),

    #[error(transparent)]
    Moon(#[from] MoonError),
}
