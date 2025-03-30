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
        error: Box<semver::Error>,
    },

    #[diagnostic(code(git::ignore::load_invalid))]
    #[error("Failed to load and parse {}.", .path.style(Style::Path))]
    IgnoreLoadFailed {
        path: PathBuf,
        #[source]
        error: Box<ignore::Error>,
    },

    #[diagnostic(code(git::repository::extract_slug))]
    #[error("Failed to extract a repository slug from Git remote candidates.")]
    ExtractRepoSlugFailed,

    #[diagnostic(code(git::worktree::parse_failed))]
    #[error("Failed to parse .git worktree file.")]
    ParseWorktreeFailed,

    #[diagnostic(code(git::worktree::load_failed))]
    #[error("Failed to load .git worktree file {}.", .path.style(Style::Path))]
    LoadWorktreeFailed {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },
}
