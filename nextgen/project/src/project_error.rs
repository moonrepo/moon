use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectError {
    #[diagnostic(code(project::task::unknown), help = "Has this task been configured?")]
    #[error(
        "Unknown task {} for project {}.",
        .task_id.style(Style::Id),
        .project_id.style(Style::Id),
    )]
    UnknownTask { task_id: Id, project_id: Id },
}
