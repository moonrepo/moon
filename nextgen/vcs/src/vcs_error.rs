use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum VcsError {
    #[diagnostic(code(git::ignore::invalid))]
    #[error("Failed to load and parse {}.", ".gitignore".style(Style::File))]
    GitignoreLoadFailed {
        #[source]
        error: ignore::Error,
    },

    #[diagnostic(code(git::repository::slug))]
    #[error("Failed to extract a repository slug from git remote candidates.")]
    GitExtractRepoSlug,
}
