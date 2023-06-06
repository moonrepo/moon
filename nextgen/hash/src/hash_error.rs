use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum HashError {
    #[diagnostic(code(hash::content::failed))]
    #[error("Failed to hash contents for {label}.")]
    ContentHashFailed {
        #[source]
        error: serde_json::Error,

        label: String,
    },
}
