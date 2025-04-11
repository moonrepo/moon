use crate::action_graph::ActionGraph;
use miette::IntoDiagnostic;
use moon_action::{
    ActionNode, InstallDependenciesNode, InstallProjectDepsNode, InstallWorkspaceDepsNode,
    RunTaskNode, SetupEnvironmentNode, SetupToolchainLegacyNode, SetupToolchainNode,
    SyncProjectNode,
};
use moon_action_context::{ActionContext, TargetState};
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use moon_app_context::AppContext;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf, is_root_level_source};
use moon_common::{Id, color};
use moon_config::{PipelineActionSwitch, TaskDependencyConfig};
use moon_pdk_api::LocateDependenciesRootInput;
use moon_platform::{PlatformManager, Runtime, ToolchainSpec};
use moon_project::Project;
use moon_query::{Criteria, build_query};
use moon_task::{Target, TargetError, TargetLocator, TargetScope, Task};
use moon_task_args::parse_task_args;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph, tasks::TaskGraphError};
use petgraph::prelude::*;
use petgraph::visit::IntoNodeReferences;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::glob::GlobSet;
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

    pub fn get_spec(
        &self,
        project: &Project,
        toolchain_id: &Id,
        allow_override: bool,
    ) -> Option<ToolchainSpec> {
        if let Some(config) = project.config.toolchain.plugins.get(toolchain_id) {
            if !config.is_enabled() {
                return None;
            }

            if allow_override {
                if let Some(version) = config.get_version() {
                    return Some(ToolchainSpec::new_override(
                        toolchain_id.to_owned(),
                        version.to_owned(),
                    ));
                }
            }
        }

        if let Some(config) = self.app_context.toolchain_config.plugins.get(toolchain_id) {
            return Some(match &config.version {
                Some(version) => ToolchainSpec::new(toolchain_id.to_owned(), version.to_owned()),
                None => ToolchainSpec::new_global(toolchain_id.to_owned()),
            });
        }

        None
    }

    #[instrument(skip_all)]
    pub async fn install_dependencies(
        &mut self,
        spec: &ToolchainSpec,
        project: &Project,
    ) -> miette::Result<Option<NodeIndex>> {
        let setup_toolchain_index = self.setup_toolchain(spec).await?;

        // Explicitly disabled
        if !self.options.install_dependencies.is_enabled(&spec.id) || spec.is_system() {
            return Ok(setup_toolchain_index);
        }

        let registry = &self.app_context.toolchain_registry;
        let toolchain = registry.load(&spec.id).await?;

        // Toolchain does not support this action, so skip and fall through
        if !toolchain.supports_tier_2().await {
            return Ok(setup_toolchain_index);
        }

        let output = toolchain
            .locate_dependencies_root(LocateDependenciesRootInput {
                context: registry.create_context(),
                starting_dir: toolchain.to_virtual_path(&project.root),
            })
            .await?;

        // Only insert this action if a root was located
        if let Some(root) = output.root {
            let abs_root = toolchain.from_virtual_path(root.any_path());
            let rel_root = abs_root
                .relative_to(&self.app_context.workspace_root)
                .into_diagnostic()?;

            // Determine if we're in the dependencies workspace
            let in_project = project.root == abs_root;
            let in_workspace = if let Some(globs) = output.members {
                if in_project {
                    true // Root always in the workspace
                } else {
                    GlobSet::new(&globs)?.matches(project.source.as_str())
                }
            } else {
                true
            };

            // If not in the dependencies workspace (if there is one),
            // or is a stand-alone project with its own lockfile,
            // we must extract the project ID and source (root)
            let (project_id, root) =
                if !in_workspace || in_project && !is_root_level_source(&project.source) {
                    (Some(project.id.clone()), project.source.clone())
                } else {
                    (None, rel_root)
                };

            let setup_env = ActionNode::setup_environment(SetupEnvironmentNode {
                project_id: project_id.clone(),
                root: root.clone(),
                toolchain_id: spec.id.clone(),
            });

            let install_deps = ActionNode::install_dependencies(InstallDependenciesNode {
                project_id,
                root,
                toolchain_id: spec.id.clone(),
            });

            // We need to conditionally create nodes and edges based on what
            // APIs have been implemented by the plugin
            let has_install_deps = toolchain.has_func("install_dependencies").await;
            let has_setup_env = toolchain.has_func("setup_environment").await;

            let index = match (has_install_deps, has_setup_env) {
                (true, true) => {
                    let setup_env_index = self.insert_node(setup_env);
                    let install_deps_index = self.insert_node(install_deps);

                    self.link_requirements(install_deps_index, vec![Some(setup_env_index)]);
                    self.link_requirements(setup_env_index, vec![setup_toolchain_index]);

                    Some(install_deps_index)
                }
                (true, false) => {
                    let install_deps_index = self.insert_node(install_deps);

                    self.link_requirements(install_deps_index, vec![setup_toolchain_index]);

                    Some(install_deps_index)
                }
                (false, true) => {
                    let setup_env_index = self.insert_node(setup_env);

                    self.link_requirements(setup_env_index, vec![setup_toolchain_index]);

                    Some(setup_env_index)
                }
                (false, false) => setup_toolchain_index,
            };

            return Ok(index);
        }

        Ok(setup_toolchain_index)
    }

    #[instrument(skip_all)]
    pub async fn setup_toolchain_legacy(
        &mut self,
        runtime: &Runtime,
    ) -> miette::Result<Option<NodeIndex>> {
        let sync_workspace_index = self.sync_workspace().await;

        // Explicitly disabled
        if !self.options.setup_toolchains.is_enabled(&runtime.toolchain) || runtime.is_system() {
            return Ok(sync_workspace_index);
        }

        let index = self.insert_node(ActionNode::setup_toolchain_legacy(
            SetupToolchainLegacyNode {
                runtime: runtime.to_owned(),
            },
        ));

        self.link_requirements(index, vec![sync_workspace_index]);

        Ok(Some(index))
    }

    #[instrument(skip_all)]
    pub async fn setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
    ) -> miette::Result<Option<NodeIndex>> {
        let sync_workspace_index = self.sync_workspace().await;

        // Explicitly disabled
        if !self.options.setup_toolchains.is_enabled(&spec.id) || spec.is_system() {
            return Ok(sync_workspace_index);
        }

        let toolchain = self.app_context.toolchain_registry.load(&spec.id).await?;

        // Toolchain does not support tier 3
        if !toolchain.supports_tier_3().await {
            return Ok(sync_workspace_index);
        }

        let index = self.insert_node(ActionNode::setup_toolchain(SetupToolchainNode {
            spec: spec.to_owned(),
        }));

        self.link_requirements(index, vec![sync_workspace_index]);

        Ok(Some(index))
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
        let sync_workspace_index = self.sync_workspace().await;

        // Explicitly disabled
        if !self.options.sync_projects.is_enabled(&project.id) {
            return Ok(sync_workspace_index);
        }

        cycle.insert(project.id.clone());

        // Determine affected state
        if let Some(affected) = &mut self.affected {
            if !affected.is_project_marked(project) {
                if let Some(by) = affected.is_project_affected(project) {
                    affected.mark_project_affected(project, by)?;
                }
            }
        }

        let mut edges = vec![sync_workspace_index];
        let index = self.insert_node(ActionNode::sync_project(SyncProjectNode {
            project_id: project.id.clone(),
        }));

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
                        edges.push(Some(dep_project_index));
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

        Some(self.insert_node(ActionNode::sync_workspace()))
    }

    // PRIVATE

    fn get_index_from_node(&self, node: &ActionNode) -> Option<NodeIndex> {
        self.graph
            .node_references()
            .find(|(_, n)| *n == node)
            .map(|(i, _)| i)
    }

    fn link_requirements(&mut self, index: NodeIndex, edges: Vec<Option<NodeIndex>>) {
        trace!(
            index = index.index(),
            requires = ?edges.iter().flat_map(|edge| edge.map(|i| i.index())).collect::<Vec<_>>(),
            "Linking requirements for index"
        );

        for edge in edges {
            // Use `update_edge` instead of `add_edge` as it avoids
            // duplicate edges from being inserted
            if let Some(edge) = edge {
                self.graph.update_edge(index, edge, ());
            }
        }
    }

    fn insert_node(&mut self, node: ActionNode) -> NodeIndex {
        if let Some(index) = self.get_index_from_node(&node) {
            return index;
        }

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
