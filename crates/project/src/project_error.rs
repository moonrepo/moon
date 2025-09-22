use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectError {
    #[diagnostic(
        code(project::unknown_file_group),
        help = "Has this group been configured?"
    )]
    #[error(
        "Unknown file group {} for project {}.",
        .group_id.style(Style::Id),
        .project_id.style(Style::Id),
    )]
    UnknownFileGroup {
        group_id: String,
        project_id: String,
    },

    #[diagnostic(code(project::unknown_task), help = "Has this task been configured?")]
    #[error(
        "Unknown task {} for project {}.",
        .task_id.style(Style::Id),
        .project_id.style(Style::Id),
    )]
    UnknownTask { task_id: String, project_id: String },
}
