use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TaskGraphError {
    #[diagnostic(code(task_graph::would_cycle))]
    #[error(
        "Unable to create task graph, adding a relationship from {} to {} would introduce a cycle.",
        .source_target.style(Style::Id),
        .target_target.style(Style::Id),
    )]
    WouldCycle {
        source_target: String,
        target_target: String,
    },
}
