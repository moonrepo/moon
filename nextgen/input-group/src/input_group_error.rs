use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum InputGroupError {
    #[diagnostic(code(input_group::missing_globs))]
    #[error("No globs defined in input group {}.", .0.style(Style::Id))]
    MissingGlobs(Id),

    #[diagnostic(code(input_group::no_tokens))]
    #[error("Token functions and variants are not supported in input groups. Received for group {}.", .0.style(Style::Id))]
    NoTokens(Id),
}
