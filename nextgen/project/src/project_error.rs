use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectError {
    #[diagnostic(code(project::invalid::env_file))]
    #[error("Failed to parse env file {}: {1}", .0.style(Style::Path))]
    InvalidEnvFile(PathBuf, String),

    #[diagnostic(code(project::missing_source))]
    #[error("No project exists at path {}.", .0.style(Style::File))]
    MissingAtSource(String),

    #[diagnostic(code(project::missing_path))]
    #[error("No project could be located starting from path {}.", .0.style(Style::Path))]
    MissingFromPath(PathBuf),

    #[diagnostic(code(project::unknown))]
    #[error("No project has been configured with the ID {}.", .0.style(Style::Id))]
    UnconfiguredID(String),

    #[diagnostic(code(project::task::unknown), help = "Has this task been configured?")]
    #[error(
        "Unknown task {} for project {}.",
        .task_id.style(Style::Id),
        .project_id.style(Style::Id),
    )]
    UnknownTask { task_id: String, project_id: String },
}
