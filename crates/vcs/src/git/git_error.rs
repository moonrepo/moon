use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum GitError {
    #[diagnostic(code(git::invalid_version))]
    #[error("Invalid or unsupported git version.")]
    InvalidVersion {
        #[source]
        error: Box<version_spec::SpecError>,
    },

    #[diagnostic(code(git::ignore::load_failed))]
    #[error("Failed to load and parse {}.", .path.style(Style::Path))]
    IgnoreLoadFailed {
        path: PathBuf,
        #[source]
        error: Box<ignore::Error>,
    },

    #[diagnostic(code(git::repository::extract_slug))]
    #[error("Failed to extract a repository slug from Git remote candidates.")]
    ExtractRepoSlugFailed,

    #[diagnostic(code(git::revision::invalid))]
    #[error(
        "Invalid Git revision {}, must not start with a dash.",
        .revision.style(Style::Hash),
    )]
    InvalidRevision { revision: String },

    #[diagnostic(code(git::repository_failed))]
    #[error("Failed to load Git repository.")]
    RepositoryLoadFailed {
        #[source]
        error: Box<gix::discover::Error>,
    },

    #[diagnostic(code(git::submodules::load_failed))]
    #[error("Failed to load Git submodules.")]
    SubmodulesLoadFailed {
        #[source]
        error: Box<gix::submodule::modules::Error>,
    },
}
