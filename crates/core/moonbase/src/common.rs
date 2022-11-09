use crate::errors::MoonbaseError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::env;

pub const LOG_TARGET: &str = "moonbase";

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Response<T> {
    Failure { message: String, status: usize },
    Success(T),
}

pub async fn post_request<I, O>(path: &str, body: I) -> Result<Response<O>, MoonbaseError>
where
    I: Serialize,
    O: DeserializeOwned,
{
    let host = env::var("MOONBASE_HOST").unwrap_or_else(|_| "https://api.moonrepo.app".to_owned());

    let body = serde_json::to_string(&body)
        .map_err(|e| MoonbaseError::JsonSerializeFailure(e.to_string()))?;

    let response = reqwest::Client::new()
        .post(format!("{}/{}", host, path))
        .body(body)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Connection", "keep-alive")
        .header("Keep-Alive", "timeout=30, max=120")
        .send()
        .await?;

    let text = response.text().await?;

    let data: Response<O> = serde_json::from_str(&text)
        .map_err(|e| MoonbaseError::JsonSerializeFailure(e.to_string()))?;

    Ok(data)
}
