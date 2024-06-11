#![allow(dead_code)]

use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum AppDockerError {
    #[diagnostic(code(app::docker::missing_manifest))]
    #[error(
        "Unable to continue, docker manifest missing. Has it been scaffolded with {}?",
        "moon docker scaffold".style(Style::Shell)
    )]
    MissingManifest,
}
