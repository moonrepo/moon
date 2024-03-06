use crate::moonbase::common::{endpoint, fetch};
use crate::moonbase_error::MoonbaseError;
use moon_time::chrono::NaiveDateTime;
use serde::{de::DeserializeOwned, Serialize};
use starbase_utils::json;

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
    let body = json::format(&query, false)
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
