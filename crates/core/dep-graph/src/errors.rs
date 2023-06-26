use miette::Diagnostic;
use moon_project::ProjectError;
use moon_query::QueryError;
use moon_target::TargetError;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum DepGraphError {
    #[error("A dependency cycle has been detected for {}.", .0.style(Style::File))]
    CycleDetected(String),

    #[error("Unknown node {0} found in dependency graph. How did this get here?")]
    UnknownNode(usize),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Project(#[from] ProjectError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Query(#[from] QueryError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Target(#[from] TargetError),
}
