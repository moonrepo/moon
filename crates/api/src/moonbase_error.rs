use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum MoonbaseError {
    #[error("Failed to check for artifact {}: {message}", .hash.style(Style::Hash))]
    ArtifactCheckFailure { hash: String, message: String },

    #[error("Failed to download artifact {}: {message}", .hash.style(Style::Hash))]
    ArtifactDownloadFailure { hash: String, message: String },

    #[error("Failed to upload artifact {}: {message}", .hash.style(Style::Hash))]
    ArtifactUploadFailure { hash: String, message: String },

    #[error("Failed to deserialize JSON response: {0}")]
    JsonDeserializeFailure(String),

    #[error("Failed to serialize JSON for request body: {0}")]
    JsonSerializeFailure(String),

    #[error("Failed to send request to moonbase: {0}")]
    Http(#[from] Box<reqwest::Error>),
}

impl From<reqwest::Error> for MoonbaseError {
    fn from(e: reqwest::Error) -> MoonbaseError {
        MoonbaseError::Http(Box::new(e))
    }
}
