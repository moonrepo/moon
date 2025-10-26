#![allow(dead_code)]

use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum AppDockerError {
    #[diagnostic(code(app::docker::missing_manifest))]
    #[error(
        "Unable to continue, docker manifest missing. Has it been scaffolded with {}?",
        "moon docker scaffold".style(Style::Shell)
    )]
    MissingManifest,

    #[diagnostic(code(app::docker::missing_file_template))]
    #[error(
        "Custom template file {} does not exist, unable to generate a {}.",
        .path.style(Style::Path),
        "Dockerfile".style(Style::File)
    )]
    MissingFileTemplate { path: PathBuf },
}
