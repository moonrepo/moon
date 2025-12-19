use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum DotEnvError {
    #[diagnostic(code(dotenv::empty_key))]
    #[error("Empty environment variable key.")]
    EmptyKey,

    #[diagnostic(code(dotenv::missing_assignment))]
    #[error("Missing `=` in environment variable assignment.")]
    MissingAssignment,

    #[error("Invalid {} format at line {line}: {message}", .path.style(Style::Path))]
    ParseFailure {
        line: usize,
        message: String,
        path: PathBuf,
    },
}
