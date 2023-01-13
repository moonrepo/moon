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

pub fn endpoint<P: AsRef<str>>(path: P) -> String {
    format!(
        "{}/{}",
        env::var("MOONBASE_HOST").unwrap_or_else(|_| "https://api.moonrepo.app".to_owned()),
        path.as_ref()
    )
}

pub fn parse_response<O>(data: String) -> Result<Response<O>, MoonbaseError>
where
    O: DeserializeOwned,
{
    serde_json::from_str(&data).map_err(|e| MoonbaseError::JsonDeserializeFailure(e.to_string()))
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
        request = request.bearer_auth(token);
    }

    let response = request.send().await?;
    let data: Response<O> = parse_response(response.text().await?)?;

    Ok(data)
}

pub async fn get_request<P, O>(path: P, token: Option<&str>) -> Result<Response<O>, MoonbaseError>
where
    P: AsRef<str>,
    O: DeserializeOwned,
{
    fetch(reqwest::Client::new().get(endpoint(path)), token).await
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

    let request = reqwest::Client::new().post(endpoint(path)).body(body);

    fetch(request, token).await
}
