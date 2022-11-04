use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use moon_logger::trace;

use crate::{
    commands::visualize::{resolver::QueryRoot, service::Service},
    helpers::{load_workspace, AnyError},
};

pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub async fn build_schema() -> Result<AppSchema, AnyError> {
    trace!("Creating state for application");
    let workspace = load_workspace().await?;
    workspace.projects.load_all()?;

    let service = Service::new(workspace);

    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(service)
        .finish();
    Ok(schema)
}
