use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CodegenError {
    #[diagnostic(code(codegen::template::exists))]
    #[error(
        "A template with the name {} already exists at {}.",
        .0.style(Style::Id),
        .1.style(Style::Path),
    )]
    ExistingTemplate(Id, PathBuf),

    #[diagnostic(code(codegen::var::parse_failed))]
    #[error("Failed to parse variable argument --{0}: {1}")]
    FailedToParseArgVar(String, String),

    #[diagnostic(code(codegen::template::missing))]
    #[error(
        "No template with the name {} could not be found at any of the configured template paths.",
        .0.style(Style::Id),
    )]
    MissingTemplate(Id),
}
