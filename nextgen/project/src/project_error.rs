use std::path::PathBuf;

use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectError {
    #[diagnostic(code(project::missing_source))]
    #[error("No project exists at path {}.", .0.style(Style::File))]
    MissingProjectAtSource(String),

    #[diagnostic(code(project::missing_path))]
    #[error("No project could be located starting from path {}.", .0.style(Style::Path))]
    MissingProjectFromPath(PathBuf),

    #[diagnostic(code(project::task::unknown), help = "Has this task been configured?")]
    #[error(
        "Unknown task {} for project {}.",
        .task_id.style(Style::Id),
        .project_id.style(Style::Id),
    )]
    UnknownTask { task_id: String, project_id: String },
}
