use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ManifestError {
    #[diagnostic(code(manifest::symlink_outside_workspace))]
    #[error(
        "Invalid task output, as the file {} is a symlink to {}, which exists outside of the workspace.",
        .output.style(Style::Path),
        .target.style(Style::Path),
    )]
    OutputSymlinkOutsideOfWorkspace { output: PathBuf, target: PathBuf },

    #[diagnostic(code(manifest::file_outside_workspace))]
    #[error(
        "Invalid task output, as the file {} exists outside of the workspace.",
        .output.style(Style::Path),
    )]
    OutputFileOutsideOfWorkspace { output: PathBuf },
}
