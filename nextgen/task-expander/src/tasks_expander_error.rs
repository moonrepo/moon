use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TasksExpanderError {
    #[diagnostic(code(task_expander::invalid_env_file))]
    #[error("Failed to parse env file {}.", .path.style(Style::Path))]
    InvalidEnvFile {
        path: PathBuf,
        #[source]
        error: dotenvy::Error,
    },
}
