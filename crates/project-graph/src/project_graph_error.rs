use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectGraphError {
    #[diagnostic(code(project_graph::no_default_project))]
    #[error("No default project has been configured.")]
    NoDefaultProject,

    #[diagnostic(code(project_graph::no_default_project))]
    #[error(
        "No default project has been configured, unable to run the task {} without a project scope.",
        .task_id.style(Style::Id),
    )]
    NoDefaultProjectForTask { task_id: String },

    #[diagnostic(code(project_graph::invalid_default_id))]
    #[error(
        "Invalid default project, no project exists with the identifier {}.",
        .id.style(Style::Id),
    )]
    InvalidDefaultId { id: String },

    #[diagnostic(code(project_graph::missing_from_path))]
    #[error("No project could be located starting from path {}.", .dir.style(Style::Path))]
    MissingFromPath { dir: PathBuf },

    #[diagnostic(code(project_graph::unknown_id))]
    #[error("No project has been configured with the identifier or alias {}.", .id.style(Style::Id))]
    UnconfiguredID { id: String },
}
