use std::collections::HashSet;

use moon_workspace::Workspace;

use super::dto::workspace_info::{WorkspaceEdgeDto, WorkspaceInfoDto, WorkspaceNodeDto};

pub async fn workspace_info(workspace: &Workspace) -> WorkspaceInfoDto {
    let labeled_graph = workspace.projects.labeled_graph();
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
