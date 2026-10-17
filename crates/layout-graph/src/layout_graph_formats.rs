use crate::layout_graph::*;
use miette::IntoDiagnostic;
use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_graph_utils::{GraphConnections, GraphConversions, GraphData, GraphToDot};
use moon_toolchain::DependenciesWorkspaceRole;
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use std::fmt;

/// A node of the layout graph, when it's rendered as a graph.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum LayoutNode {
    /// The workspace root, which links to the roots of workspace bindings.
    Workspace,
    Root {
        toolchain: Id,
        root: WorkspaceRelativePathBuf,
    },
    Project(Id),
}

impl fmt::Display for LayoutNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workspace => write!(f, "(workspace)"),
            Self::Root { toolchain, root } => write!(f, "{}", get_root_label(toolchain, root)),
            Self::Project(id) => write!(f, "{id}"),
        }
    }
}

/// An edge from a project (or the workspace) to a root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutEdge {
    Role(DependenciesWorkspaceRole),
    /// Located the root, but isn't a member of it.
    Outside,
}

impl fmt::Display for LayoutEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Role(DependenciesWorkspaceRole::WorkspaceRoot) => "workspace-root",
                Self::Role(DependenciesWorkspaceRole::WorkspaceMember) => "workspace-member",
                Self::Role(DependenciesWorkspaceRole::PackageRoot) => "package-root",
                Self::Outside => "outside",
            }
        )
    }
}

/// The layout graph as a petgraph, for formats like DOT. The workspace is the
/// first node, then each root in order, then each project by ID.
pub struct LayoutGraphView {
    graph: DiGraph<LayoutNode, LayoutEdge>,
}

impl GraphData<LayoutNode, LayoutEdge, String> for LayoutGraphView {
    type NodeWeight = LayoutNode;

    fn get_graph(&self) -> &DiGraph<LayoutNode, LayoutEdge> {
        &self.graph
    }

    fn get_nodes(&self) -> FxHashMap<NodeIndex, &LayoutNode> {
        self.graph
            .node_indices()
            .map(|index| (index, &self.graph[index]))
            .collect()
    }

    fn get_node_by_index(&self, index: &NodeIndex) -> &LayoutNode {
        &self.graph[*index]
    }

    fn get_node_key(&self, node: &LayoutNode) -> String {
        node.to_string()
    }
}

impl GraphConnections<LayoutNode, LayoutEdge, String> for LayoutGraphView {
    fn get_node_index(&self, node: &LayoutNode) -> NodeIndex {
        self.graph
            .node_indices()
            .find(|index| &self.graph[*index] == node)
            .expect("Layout node not found in graph!")
    }
}

impl GraphConversions<LayoutNode, LayoutEdge, String> for LayoutGraphView {}

impl GraphToDot<LayoutNode, LayoutEdge, String> for LayoutGraphView {}

impl LayoutGraph {
    /// Render the layout as a graph of projects and roots, where each edge
    /// is a binding, labeled with its role.
    pub fn to_view(&self) -> LayoutGraphView {
        let mut graph = DiGraph::new();
        let workspace_index = graph.add_node(LayoutNode::Workspace);

        let root_indices = self
            .roots
            .iter()
            .map(|root| {
                graph.add_node(LayoutNode::Root {
                    toolchain: root.toolchain.clone(),
                    root: root.root.clone(),
                })
            })
            .collect::<Vec<_>>();

        let get_edge = |binding: &Binding| match &binding.placement {
            Placement::Root { index, role } => {
                Some((root_indices[index.index()], LayoutEdge::Role(*role)))
            }
            Placement::Outside { root } => self
                .find_root(&binding.toolchain, root)
                .map(|index| (root_indices[index.index()], LayoutEdge::Outside)),
            Placement::None => None,
        };

        for binding in &self.workspace_bindings {
            if let Some((root_index, edge)) = get_edge(binding) {
                graph.add_edge(workspace_index, root_index, edge);
            }
        }

        for (project_id, bindings) in &self.bindings {
            let project_index = graph.add_node(LayoutNode::Project(project_id.clone()));

            for binding in bindings {
                if let Some((root_index, edge)) = get_edge(binding) {
                    graph.add_edge(project_index, root_index, edge);
                }
            }
        }

        LayoutGraphView { graph }
    }

    /// Format the layout as a DOT graph.
    pub fn to_dot(&self) -> String {
        self.to_view().to_dot()
    }

    /// Format the layout as JSON, with every root and binding.
    pub fn to_json(&self, pretty: bool) -> miette::Result<String> {
        if pretty {
            serde_json::to_string_pretty(self).into_diagnostic()
        } else {
            serde_json::to_string(self).into_diagnostic()
        }
    }
}
