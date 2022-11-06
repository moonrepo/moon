use async_graphql::{Context, Object, Result};

use super::{
    dto::{status::StatusDto, workspace_info::WorkspaceInfoDto},
    service::{Service, ServiceTrait},
};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn status<'a>(&self, ctx: &Context<'a>) -> StatusDto {
        ctx.data_unchecked::<Service>().status()
    }

    async fn workspace_info<'a>(&self, ctx: &Context<'a>) -> Result<WorkspaceInfoDto> {
        Ok(ctx.data_unchecked::<Service>().workspace_info().await)
    }
}
