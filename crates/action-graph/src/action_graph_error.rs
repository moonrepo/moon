use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ActionGraphError {
    #[diagnostic(code(action_graph::cycle_detected))]
    #[error("A dependency cycle has been detected for {}.", .0.style(Style::Label))]
    CycleDetected(String),

    #[diagnostic(
        code(action_graph::unknown_task),
        help = "Has this task been configured?"
    )]
    #[error(
        "Unknown task {} for project {}.",
        .task_id.style(Style::Id),
        .project_id.style(Style::Id),
    )]
    UnknownTask { task_id: Id, project_id: Id },
}
