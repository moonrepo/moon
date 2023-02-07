use crate::common::{endpoint, fetch};
use crate::errors::MoonbaseError;
use moon_utils::time::chrono::NaiveDateTime;
use serde::{de::DeserializeOwned, Serialize};

pub use graphql_client::{
    Error as GraphQLError, GraphQLQuery, QueryBody, Response as GraphQLResponse,
};

pub async fn post_mutation<O>(
    query: QueryBody<impl Serialize>,
    token: Option<&str>,
) -> Result<GraphQLResponse<O>, MoonbaseError>
where
    O: DeserializeOwned,
{
    let body = serde_json::to_string(&query)
        .map_err(|e| MoonbaseError::JsonSerializeFailure(e.to_string()))?;

    let request = reqwest::Client::new().post(endpoint("graphql")).body(body);

    fetch(request, token).await
}

#[derive(GraphQLQuery)]
#[graphql(schema_path = "schema.json", query_path = "mutations/create_run.gql")]
pub struct CreateRun;

#[derive(GraphQLQuery)]
#[graphql(schema_path = "schema.json", query_path = "mutations/update_run.gql")]
pub struct UpdateRun;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "mutations/add_job_to_run.gql"
)]
pub struct AddJobToRun;

#[derive(GraphQLQuery)]
#[graphql(schema_path = "schema.json", query_path = "mutations/update_job.gql")]
pub struct UpdateJob;
