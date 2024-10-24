use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum WorkspaceBuilderError {
    #[diagnostic(code(project_graph::duplicate_alias))]
    #[error(
        "Project {} is already using the alias {}, unable to use the alias for project {}.\nTry changing the alias to something unique to move forward.",
        .old_id.style(Style::Id),
        .alias.style(Style::Label),
        .new_id.style(Style::Id),
    )]
    DuplicateProjectAlias {
        alias: String,
        old_id: Id,
        new_id: Id,
    },

    #[diagnostic(code(project_graph::duplicate_id))]
    #[error(
        "A project already exists with the identifier {} (existing source {}, new source {}).\nTry renaming the project folder to make it unique, or configure the {} setting in {}.",
        .id.style(Style::Id),
        .old_source.style(Style::File),
        .new_source.style(Style::File),
        "id".style(Style::Property),
        "moon.yml".style(Style::File)
    )]
    DuplicateProjectId {
        id: Id,
        old_source: String,
        new_source: String,
    },

    #[diagnostic(code(project_graph::missing_source))]
    #[error("No project exists at source path {}.", .0.style(Style::File))]
    MissingProjectAtSource(String),
}
