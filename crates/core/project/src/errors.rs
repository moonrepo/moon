use moon_common::consts::CONFIG_PROJECT_FILENAME;
use moon_common::IdError;
use moon_error::MoonError;
use moon_file_group::FileGroupError;
use moon_query::QueryError;
use moon_target::TargetError;
use moon_task::TaskError;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error(
        "Failed to validate {}/{} configuration file.\n\n{1}",
        .0.style(Style::File),
        CONFIG_PROJECT_FILENAME.style(Style::File)
    )]
    InvalidConfigFile(String, String),

    #[error("No project exists at path {}.", .0.style(Style::File))]
    MissingProjectAtSource(String),

    #[error("No project could be located starting from path {}.", .0.style(Style::Path))]
    MissingProjectFromPath(PathBuf),

    #[error("No project has been configured with the ID {}.", .0.style(Style::Id))]
    UnconfiguredID(String),

    #[error("Task {} has not been configured for project {}.", .0.style(Style::Id), .1.style(Style::Id))]
    UnconfiguredTask(String, String),

    #[error(transparent)]
    FileGroup(#[from] FileGroupError),

    #[error(transparent)]
    Id(#[from] IdError),

    #[error(transparent)]
    Moon(#[from] MoonError),

    #[error(transparent)]
    Query(#[from] QueryError),

    #[error(transparent)]
    Target(#[from] TargetError),

    #[error(transparent)]
    Task(#[from] TaskError),
}
