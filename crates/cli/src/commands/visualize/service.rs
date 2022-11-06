use std::collections::HashSet;

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
        let labeled_graph = self.workspace.projects.labeled_graph();
        let edges = labeled_graph
            .raw_edges()
            .iter()
            .map(|e| WorkspaceEdgeDto {
                source: e.source().index(),
                target: e.target().index(),
                id: format!("{}->{}", e.source().index(), e.target().index()),
            })
            .collect::<Vec<_>>();
        let mut nodes = HashSet::new();
        for edge in labeled_graph.raw_edges().iter() {
            let source = labeled_graph
                .node_weight(edge.source())
                .expect("Unable to get node")
                .clone();
            let target = labeled_graph
                .node_weight(edge.target())
                .expect("Unable to get node")
                .clone();
            nodes.insert(WorkspaceNodeDto {
                id: edge.source().index(),
                label: source,
            });
            nodes.insert(WorkspaceNodeDto {
                id: edge.target().index(),
                label: target,
            });
        }
        let nodes = nodes.into_iter().collect();
        WorkspaceInfoDto { edges, nodes }
    }
}
