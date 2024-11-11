use miette::Diagnostic;
use moon_common::{Style, Stylize};
use moon_task::Target;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TaskGraphError {
    #[diagnostic(
        code(task_graph::unknown_target),
        help = "Has this task been configured?"
    )]
    #[error(
        "Unknown task {} for project {}.",
        .0.task_id.style(Style::Id),
        .0.get_project_id().unwrap().style(Style::Id),
    )]
    UnconfiguredTarget(Target),
}
