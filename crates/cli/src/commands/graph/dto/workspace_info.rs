use moon_config::ProjectID;
use serde::{Deserialize, Serialize};

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceNodeDto {
    pub id: usize,
    pub label: String,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceEdgeDto {
    pub id: ProjectID,
    pub source: usize,
    pub target: usize,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceInfoDto {
    pub nodes: Vec<WorkspaceNodeDto>,
    pub edges: Vec<WorkspaceEdgeDto>,
}
