use thiserror::Error;

#[derive(Error, Debug)]
pub enum MoonbaseError {
    #[error("Failed to deserialize JSON response. {0}")]
    JsonDeserializeFailure(String),

    #[error("Failed to serialize JSON for request body. {0}")]
    JsonSerializeFailure(String),

    #[error("Failed to send request to moonbase. {0}")]
    Http(#[from] reqwest::Error),
}
