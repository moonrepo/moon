use miette::Diagnostic;
use moon_common::{Style, Stylize};
use moon_task::Target;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TaskGraphError {
    #[diagnostic(code(task_graph::unknown_target))]
    #[error("No task has been configured with the target {}.", .0.style(Style::Id))]
    UnconfiguredTarget(Target),
}
