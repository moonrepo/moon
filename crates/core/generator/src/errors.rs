use miette::Diagnostic;
use moon_common::consts;
use moon_config::ConfigError;
use starbase_styles::{Style, Stylize};
use starbase_utils::{fs::FsError, json::JsonError, yaml::YamlError};
use std::path::PathBuf;
use tera::Error as TeraError;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum GeneratorError {
    #[error("A template with the name {} already exists at {}.", .0.style(Style::Id), .1.style(Style::Path))]
    ExistingTemplate(String, PathBuf),

    #[error("Failed to parse variable argument --{0}: {1}")]
    FailedToParseArgVar(String, String),

    #[error(
        "Failed to validate {} schema.\n\n{0}",
        consts::CONFIG_WORKSPACE_FILENAME.style(Style::File)
    )]
    InvalidConfigFile(String),

    #[error("No template with the name {} could not be found at any of the configured template paths.", .0.style(Style::Id))]
    MissingTemplate(String),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Tera(#[from] TeraError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] FsError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Json(#[from] JsonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Yaml(#[from] YamlError),
}
