use async_graphql::SimpleObject;

#[derive(SimpleObject)]
pub struct StatusDto {
    pub is_running: bool,
}
