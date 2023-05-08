use moon_common::{Diagnostic, IdError, Style, Stylize};
use starbase_utils::glob::GlobError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileGroupError {
    #[error("No globs defined in file group {}.", .0.style(Style::Id))]
    NoGlobs(String),

    #[error(transparent)]
    Glob(#[from] GlobError),

    #[error(transparent)]
    Id(#[from] IdError),
}

impl Diagnostic for FileGroupError {}
