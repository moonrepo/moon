use crate::common::{endpoint, fetch};
use crate::errors::MoonbaseError;
use moon_utils::time::chrono::NaiveDateTime;
use rustc_hash::FxHashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// This represents server (GraphqlError) and client (UserError) errors!
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphqlError {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphqlResponse<T> {
    pub data: T,
    pub errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationRequest<T> {
    pub query: String,
    pub variables: T,
}

pub async fn post_mutation<M, V, O>(
    query: M,
    input: V,
    token: Option<&str>,
) -> Result<GraphqlResponse<O>, MoonbaseError>
where
    M: AsRef<str>,
    V: Serialize,
    O: DeserializeOwned,
{
    let body = serde_json::to_string(&MutationRequest {
        query: query.as_ref().to_owned(),
        variables: FxHashMap::from_iter([("input".to_owned(), input)]),
    })
    .map_err(|e| MoonbaseError::JsonSerializeFailure(e.to_string()))?;

    let request = reqwest::Client::new().post(endpoint("graphql")).body(body);

    fetch(request, token).await
}

// We don't need all fields, just the ID
#[derive(Debug, Deserialize, Serialize)]
pub struct GenericRecord {
    pub id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BigGenericRecord {
    pub id: i64,
}

// RUNS

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
    pub run: Option<BigGenericRecord>,
    pub user_errors: Vec<GraphqlError>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunResponse {
    pub create_run: CreateRunPayload,
}

// JOBS

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobInput {
    pub run_id: i64,
    pub action: String,
    pub started_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobPayload {
    pub job: Option<BigGenericRecord>,
    pub user_errors: Vec<GraphqlError>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobResponse {
    pub add_job_to_run: CreateJobPayload,
}
