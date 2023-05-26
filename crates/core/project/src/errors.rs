use miette::Diagnostic;
use moon_common::consts::CONFIG_PROJECT_FILENAME;
use moon_common::IdError;
use moon_config2::ConfigError;
use moon_error::MoonError;
use moon_file_group::FileGroupError;
use moon_query::QueryError;
use moon_target::TargetError;
use moon_task::TaskError;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
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

    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    FileGroup(#[from] FileGroupError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Id(#[from] IdError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Moon(#[from] MoonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Query(#[from] QueryError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Target(#[from] TargetError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Task(#[from] TaskError),
}
