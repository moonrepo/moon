use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum HashError {
    #[diagnostic(code(hash::invalid_content_hash))]
    #[error(
        "Invalid SHA256 content hash: expected 64 hex characters, got {}.",
        .hash.style(Style::Symbol),
    )]
    InvalidContentHash { hash: String },
}
