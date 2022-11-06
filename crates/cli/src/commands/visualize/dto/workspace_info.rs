use async_graphql::SimpleObject;
use moon_config::ProjectID;

#[derive(Hash, Eq, PartialEq, SimpleObject)]
pub struct WorkspaceNodeDto {
    pub id: usize,
    pub label: String,
}

#[derive(SimpleObject)]
pub struct WorkspaceEdgeDto {
    pub id: ProjectID,
    pub source: usize,
    pub target: usize,
}

#[derive(SimpleObject)]
pub struct WorkspaceInfoDto {
    pub nodes: Vec<WorkspaceNodeDto>,
    pub edges: Vec<WorkspaceEdgeDto>,
}
