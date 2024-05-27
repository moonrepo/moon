use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ActionGraphError {
    #[diagnostic(code(action_graph::cycle_detected))]
    #[error("A dependency cycle has been detected for {}.", .0.style(Style::Label))]
    CycleDetected(String),
}
