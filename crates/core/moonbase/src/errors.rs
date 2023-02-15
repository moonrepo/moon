use moon_error::MoonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MoonbaseError {
    #[error("Failed to check for artifact {0}: {1}")]
    ArtifactCheckFailure(String, String),

    #[error("Failed to download artifact {0}: {1}")]
    ArtifactDownloadFailure(String, String),

    #[error("Failed to upload artifact {0}: {1}")]
    ArtifactUploadFailure(String, String),

    #[error("Failed to deserialize JSON response: {0}")]
    JsonDeserializeFailure(String),

    #[error("Failed to serialize JSON for request body: {0}")]
    JsonSerializeFailure(String),

    #[error("Failed to send request to moonbase: {0}")]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Moon(#[from] MoonError),
}
