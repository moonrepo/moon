use miette::Diagnostic;
use moon_common::{Style, Stylize};
use moon_process::ProcessError;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum VcsError {
    #[diagnostic(code(git::ignore::invalid))]
    #[error("Failed to load and parse {}.", ".gitignore".style(Style::File))]
    LoadGitignoreFailed {
        #[source]
        error: ignore::Error,
    },

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] ProcessError),
}
