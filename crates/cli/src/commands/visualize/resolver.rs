use async_graphql::{Context, Object};

use super::{
    dto::StatusDto,
    service::{Service, ServiceTrait},
};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn status<'a>(&self, ctx: &Context<'a>) -> StatusDto {
        ctx.data_unchecked::<Service>().status()
    }
}
