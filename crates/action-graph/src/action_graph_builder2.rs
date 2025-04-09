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

// sync_workspace
//   - change workspace/root files
//   - change toolchain files
// sync_project
//   - change project files/manifests
//   - change toolchain files
// install_deps:
// setup_toolchain:
// run_task:
pub struct ActionGraphBuilder {
    all_query: Option<Criteria<'static>>,
    app_context: Arc<AppContext>,
    graph: DiGraph<ActionNode, ()>,
    options: ActionGraphBuilderOptions,
    workspace_graph: Arc<WorkspaceGraph>,

    // Affected tracking
    affected: Option<AffectedTracker>,
    touched_files: Option<FxHashSet<WorkspaceRelativePathBuf>>,
}

impl ActionGraphBuilder {
    pub fn new(
        app_context: Arc<AppContext>,
        workspace_graph: Arc<WorkspaceGraph>,
        options: ActionGraphBuilderOptions,
    ) -> miette::Result<Self> {
        debug!("Building action graph");

        Ok(ActionGraphBuilder {
            affected: None,
            all_query: None,
            app_context,
            graph: DiGraph::new(),
            // initial_targets: FxHashSet::default(),
            options,
            // passthrough_targets: FxHashSet::default(),
            // platform_manager,
            // primary_targets: FxHashSet::default(),
            touched_files: None,
            workspace_graph,
        })
    }

    pub fn build(self) -> ActionGraph {
        ActionGraph::new(self.graph)
    }

    #[instrument(skip_all)]
    pub async fn setup_toolchain_legacy(&mut self, runtime: &Runtime) -> Option<NodeIndex> {
        if !self.options.setup_toolchains.is_enabled(&runtime.toolchain) || runtime.is_system() {
            return None;
        }

        let node = ActionNode::setup_toolchain(SetupToolchainNode {
            runtime: runtime.to_owned(),
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return Some(index);
        }

        let sync_workspace_index = self.sync_workspace().await;
        let index = self.insert_node(node);

        if let Some(edge) = sync_workspace_index {
            self.link_requirements(index, vec![edge]);
        }

        Some(index)
    }

    #[instrument(skip_all)]
    pub async fn setup_toolchain(&mut self, spec: &ToolchainSpec) -> Option<NodeIndex> {
        if !self.options.setup_toolchains.is_enabled(&spec.id) || spec.is_system() {
            return None;
        }

        let node = ActionNode::setup_toolchain_plugin(SetupToolchainPluginNode {
            spec: spec.to_owned(),
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return Some(index);
        }

        let sync_workspace_index = self.sync_workspace().await;
        let index = self.insert_node(node);

        if let Some(edge) = sync_workspace_index {
            self.link_requirements(index, vec![edge]);
        }

        Some(index)
    }

    #[instrument(skip_all)]
    pub async fn sync_project(&mut self, project: &Project) -> miette::Result<Option<NodeIndex>> {
        self.internal_sync_project(project, &mut FxHashSet::default())
            .await
    }

    async fn internal_sync_project(
        &mut self,
        project: &Project,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<Option<NodeIndex>> {
        if !self.options.sync_projects.is_enabled(&project.id) {
            return Ok(None);
        }

        let node = ActionNode::sync_project(SyncProjectNode {
            project_id: project.id.clone(),
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(index));
        }

        cycle.insert(project.id.clone());

        // Determine affected state
        if let Some(affected) = &mut self.affected {
            if let Some(by) = affected.is_project_affected(project) {
                affected.mark_project_affected(project, by)?;
            }
        }

        let mut edges = vec![];

        if let Some(edge) = self.sync_workspace().await {
            edges.push(edge);
        }

        let index = self.insert_node(node);

        // And we should also depend on other projects
        if self.options.sync_project_dependencies {
            for dep_project_id in self.workspace_graph.projects.dependencies_of(project) {
                if cycle.contains(&dep_project_id) {
                    continue;
                }

                let dep_project = self.workspace_graph.get_project(&dep_project_id)?;

                if let Some(dep_project_index) =
                    Box::pin(self.internal_sync_project(&dep_project, cycle)).await?
                {
                    if index != dep_project_index {
                        edges.push(dep_project_index);
                    }
                }
            }
        }

        if !edges.is_empty() {
            self.link_requirements(index, edges);
        }

        Ok(Some(index))
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
