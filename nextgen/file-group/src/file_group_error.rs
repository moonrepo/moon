use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum FileGroupError {
    #[diagnostic(code(file_group::missing_globs))]
    #[error("No globs defined in file group {}.", .0.style(Style::Id))]
    NoGlobs(String),
}
