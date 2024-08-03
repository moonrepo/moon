use miette::Diagnostic;
use moon_common::{get_config_file_label, Id, Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectGraphError {
    #[diagnostic(code(project_graph::duplicate_alias))]
    #[error(
        "Project {} is already using the alias {}, unable to set the alias for project {}.\nTry changing the alias to something unique to move forward.",
        .old_id.style(Style::Id),
        .alias.style(Style::Label),
        .new_id.style(Style::Id),
    )]
    DuplicateAlias {
        alias: String,
        old_id: Id,
        new_id: Id,
    },

    #[diagnostic(code(project_graph::duplicate_id))]
    #[error(
        "A project already exists with the name {} (existing source {}, new source {}).\nTry renaming the project folder to make it unique, or configure the {} setting in {}.",
        .id.style(Style::Id),
        .old_source.style(Style::File),
        .new_source.style(Style::File),
        "id".style(Style::Property),
        get_config_file_label("moon", false).style(Style::File)
    )]
    DuplicateId {
        id: Id,
        old_source: String,
        new_source: String,
    },

    #[diagnostic(code(project_graph::missing_source))]
    #[error("No project exists at source path {}.", .0.style(Style::File))]
    MissingAtSource(String),

    #[diagnostic(code(project_graph::missing_from_path))]
    #[error("No project could be located starting from path {}.", .0.style(Style::Path))]
    MissingFromPath(PathBuf),

    #[diagnostic(code(project_graph::unknown_project))]
    #[error("No project has been configured with the name or alias {}.", .0.style(Style::Id))]
    UnconfiguredID(Id),
}
