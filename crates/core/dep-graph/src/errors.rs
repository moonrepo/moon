use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum DepGraphError {
    #[error("A dependency cycle has been detected for {}.", .0.style(Style::File))]
    CycleDetected(String),

    #[error("Unknown node {0} found in dependency graph. How did this get here?")]
    UnknownNode(usize),
}
