use async_graphql::{Context, Object};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn hello<'a>(&self, _ctx: &Context<'a>) -> &'static str {
        "hello world!"
    }
}
