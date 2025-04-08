use crate::action_graph::ActionGraph;
use moon_action::{
    ActionNode, InstallProjectDepsNode, InstallWorkspaceDepsNode, RunTaskNode, SetupToolchainNode,
    SetupToolchainPluginNode, SyncProjectNode,
};
use moon_action_context::{ActionContext, TargetState};
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use moon_app_context::AppContext;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, color};
use moon_config::{PipelineActionSwitch, TaskDependencyConfig};
use moon_platform::{PlatformManager, Runtime, ToolchainSpec};
use moon_project::Project;
use moon_query::{Criteria, build_query};
use moon_task::{Target, TargetError, TargetLocator, TargetScope, Task};
use moon_task_args::parse_task_args;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph, tasks::TaskGraphError};
use petgraph::prelude::*;
use petgraph::visit::IntoNodeReferences;
use rustc_hash::{FxHashMap, FxHashSet};
use std::mem;
use std::sync::Arc;
use tracing::{debug, instrument, trace};

pub struct ActionGraphBuilderOptions {
    pub install_dependencies: PipelineActionSwitch,
    pub setup_toolchains: PipelineActionSwitch,
    pub sync_projects: PipelineActionSwitch,
    pub sync_project_dependencies: bool,
    pub sync_workspace: bool,
}

impl Default for ActionGraphBuilderOptions {
    fn default() -> Self {
        Self::new(true)
    }
}

impl ActionGraphBuilderOptions {
    pub fn new(state: bool) -> Self {
        Self {
            install_dependencies: state.into(),
            setup_toolchains: state.into(),
            sync_projects: state.into(),
            sync_project_dependencies: state,
            sync_workspace: state,
        }
    }
}

pub struct ActionGraphBuilder {
    all_query: Option<Criteria<'static>>,
    app_context: Arc<AppContext>,
    graph: DiGraph<ActionNode, ()>,
    options: ActionGraphBuilderOptions,
    workspace_graph: Arc<WorkspaceGraph>,
}

impl ActionGraphBuilder {
    pub fn new(
        app_context: Arc<AppContext>,
        workspace_graph: Arc<WorkspaceGraph>,
        options: ActionGraphBuilderOptions,
    ) -> miette::Result<Self> {
        debug!("Building action graph");

        Ok(ActionGraphBuilder {
            all_query: None,
            // affected: None,
            app_context,
            graph: DiGraph::new(),
            // initial_targets: FxHashSet::default(),
            options,
            // passthrough_targets: FxHashSet::default(),
            // platform_manager,
            // primary_targets: FxHashSet::default(),
            workspace_graph,
            // touched_files: None,
        })
    }

    pub fn build(self) -> ActionGraph {
        ActionGraph::new(self.graph)
    }

    pub async fn sync_workspace(&mut self) -> Option<NodeIndex> {
        if !self.options.sync_workspace {
            return None;
        }

        let node = ActionNode::sync_workspace();

        if let Some(index) = self.get_index_from_node(&node) {
            return Some(index);
        }

        Some(self.insert_node(node))
    }

    // PRIVATE

    fn get_index_from_node(&self, node: &ActionNode) -> Option<NodeIndex> {
        self.graph
            .node_references()
            .find(|(_, n)| *n == node)
            .map(|(i, _)| i)
    }

    fn link_requirements(&mut self, index: NodeIndex, edges: Vec<NodeIndex>) {
        trace!(
            index = index.index(),
            requires = ?edges.iter().map(|i| i.index()).collect::<Vec<_>>(),
            "Linking requirements for index"
        );

        for edge in edges {
            // Use `update_edge` instead of `add_edge` as it avoids
            // duplicate edges from being inserted
            self.graph.update_edge(index, edge, ());
        }
    }

    fn insert_node(&mut self, node: ActionNode) -> NodeIndex {
        let label = node.label();
        let index = self.graph.add_node(node);

        debug!(
            index = index.index(),
            "Adding {} to graph",
            color::muted_light(label)
        );

        index
    }
}
