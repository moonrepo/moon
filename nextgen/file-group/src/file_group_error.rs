use miette::Diagnostic;
use moon_common::{IdError, Style, Stylize};
use starbase_utils::glob::GlobError;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum FileGroupError {
    #[error("No globs defined in file group {}.", .0.style(Style::Id))]
    NoGlobs(String),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Glob(#[from] GlobError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Id(#[from] IdError),
}
