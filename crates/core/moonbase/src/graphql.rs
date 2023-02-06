use crate::common::{endpoint, fetch, Response};
use crate::errors::MoonbaseError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserError {
    pub code: Option<String>,
    pub message: String,
    pub path: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationRequest<T> {
    pub query: String,
    pub variables: T,
}

pub async fn post_mutation<M, V, O>(
    query: M,
    variables: V,
    token: Option<&str>,
) -> Result<Response<O>, MoonbaseError>
where
    M: AsRef<str>,
    V: Serialize,
    O: DeserializeOwned,
{
    let body = serde_json::to_string(&MutationRequest {
        query: query.as_ref().to_owned(),
        variables,
    })
    .map_err(|e| MoonbaseError::JsonSerializeFailure(e.to_string()))?;

    let request = reqwest::Client::new().post(endpoint("graphql")).body(body);

    fetch(request, token).await
}

// We don't need all fields, just the ID
#[derive(Debug, Deserialize, Serialize)]
pub struct GenericRecord {
    id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunInput {
    pub branch: String,
    pub job_count: usize,
    pub repository_id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunPayload {
    pub run: Option<GenericRecord>,
    pub user_errors: Vec<UserError>,
}
