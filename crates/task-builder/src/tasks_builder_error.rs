use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TasksBuilderError {
    #[diagnostic(
        code(task_builder::unknown_extends),
        help = "Has the task been renamed or excluded?"
    )]
    #[error(
        "Task {} is extending an unknown task {}.",
        .source_id.style(Style::Id),
        .target_id.style(Style::Id),
    )]
    UnknownExtendsSource { source_id: Id, target_id: Id },
}
