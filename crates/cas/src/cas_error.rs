use miette::Diagnostic;
use moon_common::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CasError {
    #[diagnostic(code(cas::not_found))]
    #[error("Blob not found in CAS store for content hash {}.", .hash.style(Style::Symbol))]
    NotFound { hash: String },

    #[diagnostic(code(cas::integrity_mismatch))]
    #[error(
        "Integrity check failed for blob at {}: expected {}, computed {}.",
        .path.style(Style::Path),
        .expected.style(Style::Symbol),
        .actual.style(Style::Symbol)
    )]
    IntegrityMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[diagnostic(code(cas::write_failed))]
    #[error("Failed to write blob to CAS store at {}.", .path.style(Style::Path))]
    WriteFailed {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(cas::read_failed))]
    #[error("Failed to read blob from CAS store at {}.", .path.style(Style::Path))]
    ReadFailed {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(cas::hash_failed))]
    #[error("Failed to hash file {}.", .path.style(Style::Path))]
    HashFailed {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },
}
