use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum FileGroupError {
    #[diagnostic(code(file_group::missing_globs))]
    #[error("No globs defined in file group {}.", .0.style(Style::Id))]
    MissingGlobs(Id),

    #[diagnostic(code(file_group::no_tokens))]
    #[error("Token functions and variables are not supported in file groups. Received for group {}.", .0.style(Style::Id))]
    NoTokens(Id),
}
