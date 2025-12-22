use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum DotEnvError {
    #[diagnostic(code(dotenv::empty_key))]
    #[error("Empty environment variable key.")]
    EmptyKey,

    #[diagnostic(code(dotenv::invalid_key))]
    #[error(
        "Invalid environment variable key {}, must contain alphanumeric characters and underscores.",
        .key.style(Style::Symbol)
    )]
    InvalidKey { key: String },

    #[diagnostic(code(dotenv::invalid_key_prefix))]
    #[error(
        "Invalid environment variable key {}, must start with an alphabetic character or underscore.",
        .key.style(Style::Symbol)
    )]
    InvalidKeyPrefix { key: String },

    #[diagnostic(code(dotenv::missing_assignment))]
    #[error("Missing `=` in environment variable assignment.")]
    MissingAssignment,

    #[error("Failed to parse env file {} at line {line}: {message}", .path.style(Style::Path))]
    ParseFailure {
        line: usize,
        message: String,
        path: PathBuf,
    },
}
