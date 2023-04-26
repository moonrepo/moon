use moon_error::MoonError;
use moon_project::ProjectError;
use moon_query::QueryError;
use moon_target2::TargetError;
use moon_task::TaskError;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DepGraphError {
    #[error("A dependency cycle has been detected for {}.", .0.style(Style::File))]
    CycleDetected(String),

    #[error("Unknown node {0} found in dependency graph. How did this get here?")]
    UnknownNode(usize),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Project(#[from] ProjectError),

    #[error(transparent)]
    Query(#[from] QueryError),

    #[error(transparent)]
    Task(#[from] TaskError),

    #[error(transparent)]
    Target(#[from] TargetError),
}
