use crate::errors::MoonbaseError;
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Response<T> {
    Failure { message: String, status: usize },
    Success(T),
}

pub fn get_host() -> String {
    env::var("MOONBASE_HOST").unwrap_or_else(|_| "https://api.moonrepo.app".to_owned())
}

pub async fn fetch<O>(
    request: RequestBuilder,
    token: Option<&str>,
) -> Result<Response<O>, MoonbaseError>
where
    O: DeserializeOwned,
{
    let mut request = request
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Connection", "keep-alive")
        .header("Keep-Alive", "timeout=30, max=120");

    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().await?;
    let text = response.text().await?;

    let data: Response<O> = serde_json::from_str(&text)
        .map_err(|e| MoonbaseError::JsonDeserializeFailure(e.to_string()))?;

    Ok(data)
}

pub async fn get_request<P, O>(path: P, token: Option<&str>) -> Result<Response<O>, MoonbaseError>
where
    P: AsRef<str>,
    O: DeserializeOwned,
{
    fetch(
        reqwest::Client::new().get(format!("{}/{}", get_host(), path.as_ref())),
        token,
    )
    .await
}

pub async fn post_request<P, I, O>(
    path: P,
    body: I,
    token: Option<&str>,
) -> Result<Response<O>, MoonbaseError>
where
    P: AsRef<str>,
    I: Serialize,
    O: DeserializeOwned,
{
    let body = serde_json::to_string(&body)
        .map_err(|e| MoonbaseError::JsonSerializeFailure(e.to_string()))?;

    let request = reqwest::Client::new()
        .post(format!("{}/{}", get_host(), path.as_ref()))
        .body(body);

    fetch(request, token).await
}
