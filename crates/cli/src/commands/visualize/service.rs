use async_trait::async_trait;
use moon_workspace::Workspace;

use super::dto::{
    status::StatusDto,
    workspace_info::{WorkspaceEdgeDto, WorkspaceInfoDto, WorkspaceNodeDto},
};

#[async_trait]
pub trait ServiceTrait: Sync + Send {
    fn status(&self) -> StatusDto;
    async fn workspace_info(&self) -> WorkspaceInfoDto;
}

pub struct Service {
    pub workspace: Workspace,
}

impl Service {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl ServiceTrait for Service {
    fn status(&self) -> StatusDto {
        StatusDto { is_running: true }
    }

    async fn workspace_info(&self) -> WorkspaceInfoDto {
        let all_projects = self
            .workspace
            .projects
            .all_projects()
            .expect("Unable to get all projects");
        let nodes = all_projects
            .into_iter()
            .map(|project| WorkspaceNodeDto { id: project.id })
            .collect();
        let all_edges = self
            .workspace
            .projects
            .all_edges()
            .expect("Unable to get all edges");
        let edges = all_edges
            .into_iter()
            .map(|edge| {
                let source = edge.source;
                let target = edge.target;
                let id = format!("{}->{}", source, target);
                WorkspaceEdgeDto { id, source, target }
            })
            .collect();
        WorkspaceInfoDto { edges, nodes }
    }
}
