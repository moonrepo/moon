use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CasError {
    #[diagnostic(code(cas::invalid_hash))]
    #[error("Invalid content hash: expected 64 hex characters, got \"{hash}\"")]
    InvalidHash { hash: String },

    #[diagnostic(code(cas::not_found))]
    #[error("Blob not found in CAS store for hash {hash}")]
    NotFound { hash: String },

    #[diagnostic(code(cas::integrity_mismatch))]
    #[error(
        "Integrity check failed for blob at {}: expected {expected}, computed {actual}",
        path.display()
    )]
    IntegrityMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[diagnostic(code(cas::write_failed))]
    #[error("Failed to write blob to CAS store at {}", path.display())]
    WriteFailed {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(cas::read_failed))]
    #[error("Failed to read blob from CAS store at {}", path.display())]
    ReadFailed {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },
}
