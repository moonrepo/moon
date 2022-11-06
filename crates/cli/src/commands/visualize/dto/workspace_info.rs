use async_graphql::SimpleObject;
use moon_config::ProjectID;

#[derive(SimpleObject)]
pub struct WorkspaceNodeDto {
    pub id: ProjectID,
}

#[derive(SimpleObject)]
pub struct WorkspaceEdgeDto {
    pub id: String,
    pub source: ProjectID,
    pub target: ProjectID,
}

#[derive(SimpleObject)]
pub struct WorkspaceInfoDto {
    pub nodes: Vec<WorkspaceNodeDto>,
    pub edges: Vec<WorkspaceEdgeDto>,
}
