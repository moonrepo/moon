//! Stages 2 to 4: assemble the graph by replaying the selected events, while
//! resolving provisioning actions, then add the edges that depend on the whole
//! graph, and freeze it.
//!
//! Events are replayed in the order the action graph has always inserted nodes
//! and linked edges, so node indices are the same as always. A task's toolchains
//! are resolved (setup, environment, and dependency installs) through the
//! toolchain registry, whose plugin calls are memoized, right before its node.
//!
//! Once every event has been replayed, the edges that depend on the whole graph
//! are added: between project syncs (production and peer dependencies first,
//! then the other scopes, skipping any that would close a cycle), then the
//! present dependencies of tasks inserted without their dependencies, and last
//! the serial ordering edges, which are skipped when they would introduce a
//! cycle, and never removed.

use super::RequestId;
use super::select::{Event, ProvisionRequest, SerialLink, TaskPlan};
use super::{get_project_spec, get_spec};
use crate::action_graph::ActionGraph;
use crate::action_graph_error::ActionGraphError;
use crate::action_graph_options::ActionGraphBuilderOptions;
use daggy::{Dag, Walker};
use miette::IntoDiagnostic;
use moon_action::{
    ActionNode, InstallDependenciesNode, SetupEnvironmentNode, SetupToolchainNode, SyncProjectNode,
};
use moon_app_context::AppContext;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_common::{Id, color};
use moon_config::{TaskDependencyConfig, TaskDependencyType};
use moon_pdk_api::{DefineRequirementsInput, LocateDependenciesRootInput};
use moon_project::Project;
use moon_task::Target;
use moon_toolchain::{DependenciesWorkspace, DependenciesWorkspaceRole, ToolchainSpec};
use moon_workspace_graph::WorkspaceGraph;
use moon_workspace_graph::projects::ScopePartition;
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use tracing::{debug, trace, warn};

/// The identity of a provisioning or sync node. Task nodes are identified by
/// their instance, as instances are already unique.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum NodeKey {
    InstallDependencies {
        toolchain: Id,
        root: WorkspaceRelativePathBuf,
        members: Option<Vec<String>>,
        project: Option<Id>,
    },
    SetupEnvironment {
        toolchain: Id,
        root: WorkspaceRelativePathBuf,
        project: Option<Id>,
    },
    SetupProto,
    SetupToolchain(ToolchainSpec),
    SyncProject(Id),
    SyncWorkspace,
}

pub struct AssembledGraph {
    pub graph: ActionGraph,
    /// Dependencies of tasks inserted without them, that have no instance.
    pub ignored_dependencies: FxHashMap<Target, FxHashSet<Target>>,
    /// The node of each task instance.
    pub instance_nodes: Vec<NodeIndex>,
    /// The nodes that each provisioning or sync request produced.
    pub request_nodes: FxHashMap<RequestId, Vec<NodeIndex>>,
    /// The node of each project sync.
    pub sync_project_nodes: FxHashMap<Id, NodeIndex>,
    /// Targets depended on with `wait` by tasks inserted without dependencies.
    pub wait_targets: FxHashSet<Target>,
}

struct GraphAssembler<'app> {
    app_context: &'app AppContext,
    options: &'app ActionGraphBuilderOptions,
    workspace_graph: &'app WorkspaceGraph,

    graph: Dag<ActionNode, TaskDependencyType>,
    keys: FxHashMap<NodeKey, NodeIndex>,
    cleanup_indices: FxHashSet<NodeIndex>,

    instance_nodes: Vec<Option<NodeIndex>>,
    request_nodes: FxHashMap<RequestId, Vec<NodeIndex>>,
    /// Setup and sync edges of tasks whose dependencies are being visited.
    task_edges: FxHashMap<usize, Vec<Option<NodeIndex>>>,

    // Edges that aren't followed when walking a dependency's subtree, as
    // they don't exist yet (or never) when the serial order is applied
    present_edges: FxHashSet<EdgeIndex>,
    serial_edges: FxHashSet<EdgeIndex>,
}

pub async fn assemble(
    app_context: &AppContext,
    workspace_graph: &WorkspaceGraph,
    options: &ActionGraphBuilderOptions,
    plan: &mut TaskPlan,
) -> miette::Result<AssembledGraph> {
    let instances = std::mem::take(&mut plan.instances);

    let mut assembler = GraphAssembler {
        app_context,
        options,
        workspace_graph,
        graph: Dag::new(),
        keys: FxHashMap::default(),
        cleanup_indices: FxHashSet::default(),
        instance_nodes: vec![None; instances.len()],
        request_nodes: FxHashMap::default(),
        task_edges: FxHashMap::default(),
        present_edges: FxHashSet::default(),
        serial_edges: FxHashSet::default(),
    };

    for event in std::mem::take(&mut plan.events) {
        assembler.replay(event, &instances).await?;
    }

    assembler.link_synced_projects(&plan.synced_projects);

    let (ignored_dependencies, wait_targets) =
        assembler.link_present_dependencies(&plan.no_dependencies_tasks);

    assembler.link_serial_links(&plan.serial_links);

    let instance_nodes = assembler
        .instance_nodes
        .iter()
        .map(|index| index.expect("Every task instance has a node."))
        .collect();
    let request_nodes = std::mem::take(&mut assembler.request_nodes);
    let sync_project_nodes = assembler
        .keys
        .iter()
        .filter_map(|(key, index)| match key {
            NodeKey::SyncProject(id) => Some((id.clone(), *index)),
            _ => None,
        })
        .collect();

    Ok(AssembledGraph {
        graph: assembler.freeze(),
        ignored_dependencies,
        instance_nodes,
        request_nodes,
        sync_project_nodes,
        wait_targets,
    })
}

impl GraphAssembler<'_> {
    // EVENTS

    async fn replay(&mut self, event: Event, instances: &[ActionNode]) -> miette::Result<()> {
        match event {
            Event::SyncWorkspace => {
                self.sync_workspace();
            }
            Event::AddSyncProject(id) => {
                self.insert_node(
                    Some(NodeKey::SyncProject(id.clone())),
                    ActionNode::sync_project(SyncProjectNode { project_id: id }),
                );
            }
            Event::LinkSyncProject(id) => {
                if let Some(sync_workspace_index) = self.keys.get(&NodeKey::SyncWorkspace).copied()
                {
                    let index = self.keys[&NodeKey::SyncProject(id)];

                    self.link_required_edges(index, vec![sync_workspace_index])?;
                }
            }
            Event::EnterTask {
                instance,
                project,
                toolchains,
                sync_project,
            } => {
                let mut edges = vec![sync_project.map(|id| self.keys[&NodeKey::SyncProject(id)])];

                edges.extend(
                    self.install_dependencies_by_toolchains(&project, &toolchains)
                        .await?,
                );

                // If no edges created, we should at minimum sync the workspace
                if edges.iter().all(|edge| edge.is_none()) {
                    edges.push(self.sync_workspace());
                }

                let index = self.insert_node(None, instances[instance].clone());

                self.instance_nodes[instance] = Some(index);
                self.task_edges.insert(instance, edges);
            }
            Event::ExitTask {
                instance,
                requirements,
                cleanups,
            } => {
                let index = self.get_instance_node(instance);
                let edges = self.task_edges.remove(&instance).unwrap_or_default();

                self.link_available_edges(index, edges)?;

                for (dep_instance, weight) in requirements {
                    let dep_index = self.get_instance_node(dep_instance);

                    if !self.has_edge_of_type(index, dep_index, weight) {
                        self.link_edge(index, dep_index, weight)?;
                    }
                }

                // Cleanup actions run *after* the node they clean up, so the edge
                // points from the cleanup action to the node. The cleanup action may
                // also depend on the node in some other way, which is a separate edge
                for cleanup_instance in cleanups {
                    let cleanup_index = self.get_instance_node(cleanup_instance);

                    if !self.has_edge_of_type(cleanup_index, index, TaskDependencyType::Cleanup) {
                        self.link_edge(cleanup_index, index, TaskDependencyType::Cleanup)?;
                    }

                    self.cleanup_indices.insert(cleanup_index);
                }
            }
            Event::Request(id, request) => {
                let indices = self.provision(request).await?;

                self.request_nodes.insert(id, indices);
            }
        };

        Ok(())
    }

    async fn provision(&mut self, request: ProvisionRequest) -> miette::Result<Vec<NodeIndex>> {
        let index = match request {
            ProvisionRequest::InstallDependencies(spec, project) => {
                self.install_dependencies(&spec, Some(&project)).await?
            }
            ProvisionRequest::InstallDependenciesByProject(project) => {
                return Ok(self
                    .install_dependencies_by_toolchains(&project, &project.toolchains)
                    .await?
                    .into_iter()
                    .flatten()
                    .collect());
            }
            ProvisionRequest::InstallDependenciesRoot(spec) => {
                self.install_dependencies_root(&spec).await?
            }
            ProvisionRequest::SetupEnvironment(spec, root, project) => {
                Box::pin(self.internal_setup_environment(
                    &spec,
                    &root,
                    Some(&project),
                    FxHashSet::default(),
                ))
                .await?
            }
            ProvisionRequest::SetupProto => self.setup_proto(),
            ProvisionRequest::SetupToolchain(spec, project) => {
                self.setup_toolchain(&spec, project.as_deref()).await?
            }
            ProvisionRequest::SetupToolchainAndEnvironment(spec) => {
                let index = self.setup_toolchain(&spec, None).await?;

                if index.is_some() {
                    self.setup_environment_root(&spec).await?;
                }

                index
            }
            ProvisionRequest::SetupEnvironmentRoot(spec) => {
                self.setup_environment_root(&spec).await?
            }
            ProvisionRequest::SyncWorkspace => self.sync_workspace(),
        };

        Ok(index.into_iter().collect())
    }

    // NODES

    fn insert_node(&mut self, key: Option<NodeKey>, node: ActionNode) -> NodeIndex {
        debug!(
            index = self.graph.node_count(),
            "Adding {} to graph",
            color::muted_light(node.label())
        );

        let index = self.graph.add_node(node);

        if let Some(key) = key {
            self.keys.insert(key, index);
        }

        index
    }

    fn get_or_insert_node(&mut self, key: NodeKey, node: impl FnOnce() -> ActionNode) -> NodeIndex {
        match self.keys.get(&key) {
            Some(index) => *index,
            None => self.insert_node(Some(key), node()),
        }
    }

    fn get_instance_node(&self, instance: usize) -> NodeIndex {
        self.instance_nodes[instance].expect("Task instance has not been inserted.")
    }

    fn label(&self, index: NodeIndex) -> String {
        self.graph[index].label()
    }

    fn freeze(self) -> ActionGraph {
        ActionGraph::new_with_cleanups(self.graph.into_graph(), self.cleanup_indices)
    }

    // PROVISIONING

    fn sync_workspace(&mut self) -> Option<NodeIndex> {
        if !self.options.sync_workspace {
            return None;
        }

        Some(self.get_or_insert_node(NodeKey::SyncWorkspace, ActionNode::sync_workspace))
    }

    fn setup_proto(&mut self) -> Option<NodeIndex> {
        let version = &self.app_context.toolchains_config.proto.version;

        Some(self.get_or_insert_node(NodeKey::SetupProto, || {
            ActionNode::setup_proto(version.clone())
        }))
    }

    async fn setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> miette::Result<Option<NodeIndex>> {
        Box::pin(self.internal_setup_toolchain(spec, project, FxHashSet::default())).await
    }

    async fn internal_setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
        mut cycle: FxHashSet<Id>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.setup_toolchains.is_enabled(&spec.id)
            || spec.is_system()
            || cycle.contains(&spec.id)
        {
            return Ok(None);
        }

        let app_context = self.app_context;
        let toolchain_registry = &app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;
        let mut requirement_edges = vec![];

        cycle.insert(spec.id.clone());

        // Toolchain may depend on others
        let output = toolchain
            .define_requirements(DefineRequirementsInput {
                context: toolchain_registry.create_context(),
                toolchain_config: match project {
                    Some(project) => {
                        toolchain_registry.create_merged_config(&toolchain.id, &project.config)
                    }
                    None => toolchain_registry.create_config(&toolchain.id),
                },
            })
            .await?;

        if !output.requires.is_empty() && output.for_setup_toolchain {
            for require_id in output.requires {
                let require_id = Id::new(require_id)?;

                if require_id == spec.id || cycle.contains(&require_id) {
                    continue;
                }

                let Some(require_spec) = get_spec(app_context, &require_id, project) else {
                    return Err(ActionGraphError::MissingToolchainRequirement {
                        id: spec.id.to_string(),
                        dep_id: require_id.to_string(),
                    }
                    .into());
                };

                requirement_edges.push(
                    Box::pin(self.internal_setup_toolchain(&require_spec, project, cycle.clone()))
                        .await?,
                );
            }
        }

        // Toolchain does not support tier 3 and does not require other toolchains
        if !toolchain.supports_tier_3().await && requirement_edges.is_empty() {
            return Ok(None);
        }

        let mut edges = vec![self.sync_workspace()];

        if spec.req.is_some() || app_context.toolchains_config.requires_proto() {
            edges.push(self.setup_proto());
        }

        let index = self.get_or_insert_node(NodeKey::SetupToolchain(spec.to_owned()), || {
            ActionNode::setup_toolchain(SetupToolchainNode {
                toolchain: spec.to_owned(),
            })
        });

        // Toolchains may require each other, so a requirement that would close
        // a cycle (when the other toolchain was reached first) is skipped
        self.try_link_edges(index, requirement_edges.into_iter().flatten());
        self.link_available_edges(index, edges)?;

        Ok(Some(index))
    }

    async fn internal_setup_environment(
        &mut self,
        spec: &ToolchainSpec,
        root: &WorkspaceRelativePathBuf,
        project: Option<&Project>,
        mut cycle: FxHashSet<Id>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.setup_environment.is_enabled(&spec.id)
            || spec.is_system()
            || cycle.contains(&spec.id)
        {
            return Ok(None);
        }

        let app_context = self.app_context;
        let toolchain_registry = &app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;
        let mut edges = vec![];
        let mut env_edges = vec![];

        cycle.insert(spec.id.clone());

        // Toolchain may depend on others
        let output = toolchain
            .define_requirements(DefineRequirementsInput {
                context: toolchain_registry.create_context(),
                toolchain_config: match project {
                    Some(project) => {
                        toolchain_registry.create_merged_config(&toolchain.id, &project.config)
                    }
                    None => toolchain_registry.create_config(&toolchain.id),
                },
            })
            .await?;

        for require_id in output.requires {
            let require_id = Id::new(require_id)?;

            // Skip if itself or already in cycle
            if require_id == spec.id || cycle.contains(&require_id) {
                continue;
            }

            let Some(require_spec) = get_spec(app_context, &require_id, project) else {
                if output.for_setup_environment {
                    return Err(ActionGraphError::MissingToolchainRequirement {
                        id: spec.id.to_string(),
                        dep_id: require_id.to_string(),
                    }
                    .into());
                }

                continue;
            };

            // Requires the toolchain to be setup, not the environment!
            if output.for_setup_environment {
                edges.push(self.setup_toolchain(&require_spec, project).await?);
            }

            // Required toolchains (like Node.js for JavaScript) typically don't
            // locate a dependencies root, as this toolchain does, so their
            // environment must be setup in this toolchain's root, before its own
            if let Some(env_index) = Box::pin(self.internal_setup_environment(
                &require_spec,
                root,
                project,
                cycle.clone(),
            ))
            .await?
            {
                env_edges.push(env_index);
            }
        }

        // Toolchain does not support it
        if !toolchain.has_func("setup_environment").await
            && edges.is_empty()
            && env_edges.is_empty()
        {
            return Ok(None);
        }

        edges.push(self.sync_workspace());
        edges.push(self.setup_toolchain(spec, project).await?);

        // The node may already exist, but the edges must still be linked, as the
        // version is not part of its identity, so another version of the toolchain
        // (overridden by a project that shares the root) must also be setup first
        let project_id = project.map(|project| project.id.clone());
        let index = self.get_or_insert_node(
            NodeKey::SetupEnvironment {
                toolchain: spec.id.clone(),
                root: root.clone(),
                project: project_id.clone(),
            },
            || {
                ActionNode::setup_environment(SetupEnvironmentNode {
                    project_id,
                    root: root.clone(),
                    toolchain_id: spec.id.clone(),
                })
            },
        );

        self.link_available_edges(index, edges)?;

        // Toolchains may require each other (like Python and uv), so a required
        // environment that already depends on this one is skipped
        self.try_link_edges(index, env_edges);

        Ok(Some(index))
    }

    /// Setup the environment in the workspace root, without an associated
    /// project. This inserts no actions at all, unless the located
    /// dependencies root is the workspace root.
    async fn setup_environment_root(
        &mut self,
        spec: &ToolchainSpec,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.setup_environment.is_enabled(&spec.id) || spec.is_system() {
            return Ok(None);
        }

        // Only insert actions if the dependencies root is the workspace root
        if self
            .locate_dependencies_root(spec, None)
            .await?
            .is_none_or(|deps_workspace| deps_workspace.root != self.app_context.workspace_root)
        {
            return Ok(None);
        }

        Box::pin(self.internal_setup_environment(
            spec,
            &WorkspaceRelativePathBuf::new(),
            None,
            FxHashSet::default(),
        ))
        .await
    }

    async fn install_dependencies_by_toolchains(
        &mut self,
        project: &Project,
        toolchains: &[Id],
    ) -> miette::Result<Vec<Option<NodeIndex>>> {
        let mut indices = vec![];

        for toolchain_id in toolchains {
            if let Some(spec) = get_project_spec(self.app_context, toolchain_id, project) {
                indices.push(self.install_dependencies(&spec, Some(project)).await?);
            }
        }

        Ok(indices)
    }

    /// Resolve the "install slot" of a toolchain: install its dependencies in
    /// the closest root, or the most specific setup action that exists.
    async fn install_dependencies(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> miette::Result<Option<NodeIndex>> {
        if spec.is_system() {
            return Ok(None);
        }

        // If installing dependencies is disabled, we still need to ensure the
        // toolchain has been setup (and indirectly, the workspace synced), so
        // that its binaries are available to tasks
        if !self.options.install_dependencies.is_enabled(&spec.id)
            || self
                .app_context
                .toolchains_config
                .get_plugin_config(&spec.id)
                .is_some_and(|cfg| !cfg.install_dependencies)
        {
            return self.setup_toolchain(spec, project).await;
        }

        let sync_workspace_index = self.sync_workspace();
        let setup_toolchain_index = self.setup_toolchain(spec, project).await?;
        let app_context = self.app_context;
        let toolchain = app_context.toolchain_registry.load(&spec.id).await?;

        // Toolchain does not support this action, so skip and fall through
        if !toolchain.supports_tier_2().await {
            return Ok(setup_toolchain_index);
        }

        let target_root = match project {
            Some(project) => &project.root,
            None => &app_context.workspace_root,
        };

        // Only insert this action if a root was located
        if let Some(deps_workspace) = self.locate_dependencies_root(spec, project).await?
            && let Some(deps_role) =
                toolchain.in_dependencies_workspace(&deps_workspace, target_root)?
        {
            // The action is scoped to the located root, so that every project
            // resolving to it collapses into the same action
            let root = deps_workspace
                .root
                .relative_to(&app_context.workspace_root)
                .into_diagnostic()?;

            // Unless there's no workspace, in which case the root is the only
            // package, and the project that owns it is associated, so that
            // project-level toolchain config is passed to the plugin
            let project_id = match deps_role {
                DependenciesWorkspaceRole::PackageRoot => project.map(|project| project.id.clone()),
                _ => None,
            };

            let setup_env_index = Box::pin(self.internal_setup_environment(
                spec,
                &root,
                project_id.as_ref().and(project),
                FxHashSet::default(),
            ))
            .await?;

            // Only create this action if the plugin supports it
            if toolchain.has_func("install_dependencies").await {
                let members = deps_workspace.members;
                let index = self.get_or_insert_node(
                    NodeKey::InstallDependencies {
                        toolchain: spec.id.clone(),
                        root: root.clone(),
                        members: members.clone(),
                        project: project_id.clone(),
                    },
                    || {
                        ActionNode::install_dependencies(InstallDependenciesNode {
                            members,
                            project_id,
                            root,
                            toolchain_id: spec.id.clone(),
                        })
                    },
                );

                self.link_first_available_edge(
                    index,
                    vec![setup_env_index, setup_toolchain_index, sync_workspace_index],
                )?;

                return Ok(Some(index));
            }

            // Otherwise pass through to setup environment
            if let Some(setup_env_index) = setup_env_index {
                self.link_first_available_edge(
                    setup_env_index,
                    vec![setup_toolchain_index, sync_workspace_index],
                )?;

                return Ok(Some(setup_env_index));
            }
        }

        // Or fallback entirely to setup toolchain
        Ok(setup_toolchain_index)
    }

    /// Install dependencies in the workspace root, without an associated
    /// project, when the located dependencies root is the workspace root.
    async fn install_dependencies_root(
        &mut self,
        spec: &ToolchainSpec,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if spec.is_system()
            || !self.options.install_dependencies.is_enabled(&spec.id)
            || self
                .app_context
                .toolchains_config
                .get_plugin_config(&spec.id)
                .is_some_and(|cfg| !cfg.install_dependencies)
        {
            return Ok(None);
        }

        // Only insert actions if the dependencies root is the workspace root
        if self
            .locate_dependencies_root(spec, None)
            .await?
            .is_none_or(|deps_workspace| deps_workspace.root != self.app_context.workspace_root)
        {
            return Ok(None);
        }

        self.install_dependencies(spec, None).await
    }

    async fn locate_dependencies_root(
        &self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> miette::Result<Option<DependenciesWorkspace>> {
        let toolchain_registry = &self.app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;

        // Toolchain does not support locating a root, so return
        // an empty output instead of failing the function call
        if !toolchain.supports_tier_2().await {
            return Ok(None);
        }

        toolchain
            .locate_dependencies_root(match project {
                Some(project) => LocateDependenciesRootInput {
                    context: toolchain_registry.create_context(),
                    starting_dir: toolchain.to_virtual_path(&project.root),
                    toolchain_config: toolchain_registry
                        .create_merged_config(&toolchain.id, &project.config),
                },
                None => LocateDependenciesRootInput {
                    context: toolchain_registry.create_context(),
                    starting_dir: toolchain.to_virtual_path(&self.app_context.workspace_root),
                    toolchain_config: toolchain_registry.create_config(&toolchain.id),
                },
            })
            .await
    }

    // EDGES

    fn link_first_available_edge(
        &mut self,
        index: NodeIndex,
        edges: Vec<Option<NodeIndex>>,
    ) -> miette::Result<()> {
        if let Some(edge) = edges.into_iter().flatten().next() {
            self.link_required_edges(index, vec![edge])?;
        }

        Ok(())
    }

    fn link_available_edges(
        &mut self,
        index: NodeIndex,
        edges: Vec<Option<NodeIndex>>,
    ) -> miette::Result<()> {
        self.link_required_edges(index, edges.into_iter().flatten().collect())
    }

    fn link_required_edges(
        &mut self,
        index: NodeIndex,
        edges: Vec<NodeIndex>,
    ) -> miette::Result<()> {
        for edge in edges {
            if !self.has_edge_of_type(index, edge, TaskDependencyType::Required) {
                self.link_edge(index, edge, TaskDependencyType::Required)?;
            }
        }

        Ok(())
    }

    /// Link an edge for a real dependency, which is an error if it would
    /// introduce a cycle.
    fn link_edge(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        weight: TaskDependencyType,
    ) -> miette::Result<()> {
        if self.graph.add_edge(from, to, weight).is_err() {
            return Err(ActionGraphError::WouldCycle {
                source_action: self.label(from),
                target_action: self.label(to),
                cycle: self.find_cycle(from, to),
            }
            .into());
        }

        trace!(
            index = from.index(),
            requires = ?[to.index()],
            "Linking requirements for index"
        );

        Ok(())
    }

    /// Link edges that are a preference: an edge is skipped when a required
    /// edge already links the nodes, or when it would introduce a cycle.
    fn try_link_edges(&mut self, index: NodeIndex, edges: impl IntoIterator<Item = NodeIndex>) {
        for edge in edges {
            self.try_link_edge(index, edge);
        }
    }

    fn try_link_edge(&mut self, from: NodeIndex, to: NodeIndex) -> Option<EdgeIndex> {
        if self.has_edge_of_type(from, to, TaskDependencyType::Required) {
            return None;
        }

        match self.graph.add_edge(from, to, TaskDependencyType::Required) {
            Ok(edge_index) => {
                trace!(
                    index = from.index(),
                    requires = ?[to.index()],
                    "Linking requirements for index"
                );

                Some(edge_index)
            }
            Err(_) => {
                trace!(
                    index = from.index(),
                    requires = ?[to.index()],
                    "Not linking requirements for index, as they would introduce a cycle"
                );

                None
            }
        }
    }

    /// Whether an edge of the provided type already links the two nodes. Edges
    /// of different types may link the same nodes (a task that both requires
    /// and cleans up after another), and the pipeline honors all of them.
    fn has_edge_of_type(&self, from: NodeIndex, to: NodeIndex, weight: TaskDependencyType) -> bool {
        self.graph
            .graph()
            .edges_connecting(from, to)
            .any(|edge| *edge.weight() == weight)
    }

    /// Describe the cycle that an edge from `from` to `to` would introduce,
    /// as `to` already reaches `from`.
    fn find_cycle(&self, from: NodeIndex, to: NodeIndex) -> Option<String> {
        let graph = self.graph.graph();
        let mut parents = FxHashMap::<NodeIndex, NodeIndex>::default();
        let mut queue = VecDeque::from([to]);

        parents.insert(to, to);

        while let Some(index) = queue.pop_front() {
            if index == from {
                let mut path = vec![from];
                let mut current = from;

                while current != to {
                    current = parents[&current];
                    path.push(current);
                }

                path.reverse();
                path.insert(0, from);

                return Some(format!(
                    "The cycle is {}",
                    path.into_iter()
                        .map(|index| self.label(index))
                        .collect::<Vec<_>>()
                        .join(" → ")
                ));
            }

            for edge in graph.edges_directed(index, Direction::Outgoing) {
                if let Entry::Vacant(entry) = parents.entry(edge.target()) {
                    entry.insert(index);
                    queue.push_back(edge.target());
                }
            }
        }

        None
    }

    // WHOLE GRAPH EDGES

    /// Link each project sync to the syncs of the projects it depends on,
    /// once every project sync is known, so that the edges don't depend on
    /// which caller synced a project first. Production and peer dependencies
    /// are linked first (that partition is acyclic), then the other scopes,
    /// skipping any that would close a cycle, both in sorted order.
    fn link_synced_projects(&mut self, projects: &[Arc<Project>]) {
        if !self.options.sync_project_dependencies || projects.is_empty() {
            return;
        }

        let mut projects = projects.iter().collect::<Vec<_>>();

        projects.sort_by(|a, b| a.id.cmp(&b.id));

        for partition in [ScopePartition::Production, ScopePartition::Development] {
            let mut edges = vec![];

            for project in &projects {
                let index = self.keys[&NodeKey::SyncProject(project.id.clone())];
                let mut dep_ids = self
                    .workspace_graph
                    .projects
                    .partitioned_dependencies_of(project, partition);

                dep_ids.sort();
                dep_ids.dedup();

                for dep_id in dep_ids {
                    if dep_id == project.id {
                        continue;
                    }

                    if let Some(dep_index) = self.keys.get(&NodeKey::SyncProject(dep_id)).copied()
                        && !self.has_edge_of_type(index, dep_index, TaskDependencyType::Required)
                    {
                        edges.push((index, dep_index));
                    }
                }
            }

            // Production and peer dependencies are acyclic, and project syncs
            // otherwise only link to the workspace sync, so they're added at
            // once, with a single cycle check, instead of one for each edge
            if matches!(partition, ScopePartition::Production)
                && self
                    .graph
                    .add_edges(
                        edges
                            .iter()
                            .map(|(from, to)| (*from, *to, TaskDependencyType::Required)),
                    )
                    .is_ok()
            {
                continue;
            }

            for (from, to) in edges {
                self.try_link_edge(from, to);
            }
        }
    }

    /// Link the dependencies of tasks that were inserted without them
    /// (`no_dependencies`) to the instances that exist, as the ordering between
    /// tasks that do run must still be honored. When a dependency has multiple
    /// instances (different args or env), the ones that match the dependency's
    /// config are preferred, otherwise all of them are linked. Dependencies
    /// without an instance are ignored, so that the task doesn't wait on (or
    /// hash) them. Serial ordering is not applied to these edges.
    fn link_present_dependencies(
        &mut self,
        no_dependencies_tasks: &FxHashMap<Target, Vec<TaskDependencyConfig>>,
    ) -> (FxHashMap<Target, FxHashSet<Target>>, FxHashSet<Target>) {
        let mut ignored_dependencies = FxHashMap::<Target, FxHashSet<Target>>::default();
        let mut wait_targets = FxHashSet::default();

        if no_dependencies_tasks.is_empty() {
            return (ignored_dependencies, wait_targets);
        }

        let mut target_indices = FxHashMap::<Target, Vec<NodeIndex>>::default();

        for index in self.graph.graph().node_indices() {
            if let ActionNode::RunTask(inner) = &self.graph[index] {
                target_indices
                    .entry(inner.target.clone())
                    .or_default()
                    .push(index);
            }
        }

        let mut tasks = no_dependencies_tasks.iter().collect::<Vec<_>>();

        // Link in a deterministic order, so that the graph's edge order is stable
        tasks.sort_by(|a, b| a.0.cmp(b.0));

        for (target, deps) in tasks {
            let Some(owner_indices) = target_indices.get(target) else {
                continue;
            };

            for dep in deps {
                let Some(dep_indices) = target_indices.get(&dep.target) else {
                    ignored_dependencies
                        .entry(target.clone())
                        .or_default()
                        .insert(dep.target.clone());

                    continue;
                };

                // The optional type is only a marker within the task
                // graph, the action graph has always used required
                let weight = if dep.type_of.is_required_type() {
                    TaskDependencyType::Required
                } else {
                    dep.type_of
                };

                let matching_indices = dep_indices
                    .iter()
                    .copied()
                    .filter(|index| {
                        matches!(
                            &self.graph[*index],
                            ActionNode::RunTask(inner)
                                if inner.args == dep.args && inner.env == dep.env
                        )
                    })
                    .collect::<Vec<_>>();

                let dep_indices = if matching_indices.is_empty() {
                    dep_indices.clone()
                } else {
                    matching_indices
                };

                if matches!(weight, TaskDependencyType::Wait) {
                    wait_targets.insert(dep.target.clone());
                }

                for owner_index in owner_indices {
                    if dep_indices
                        .iter()
                        .any(|dep_index| self.has_edge_of_type(*owner_index, *dep_index, weight))
                    {
                        continue;
                    }

                    for dep_index in &dep_indices {
                        if owner_index == dep_index {
                            continue;
                        }

                        match self.graph.add_edge(*owner_index, *dep_index, weight) {
                            Ok(edge_index) => {
                                self.present_edges.insert(edge_index);
                            }
                            // The task graph is acyclic, so this is unexpected
                            Err(_) => {
                                warn!(
                                    task_target = target.as_str(),
                                    dependency_target = dep.target.as_str(),
                                    "Unable to link dependency: adding a relationship from action {} to {} would introduce a cycle",
                                    self.label(*owner_index),
                                    self.label(*dep_index),
                                );
                            }
                        };
                    }
                }
            }
        }

        (ignored_dependencies, wait_targets)
    }

    /// Apply the serial ordering links, in the order that they were made.
    fn link_serial_links(&mut self, links: &[SerialLink]) {
        for link in links {
            self.link_serial_required_edges(
                self.get_instance_node(link.index),
                link.previous
                    .map(|instance| self.get_instance_node(instance)),
                link.previous_standard
                    .map(|instance| self.get_instance_node(instance)),
            );
        }
    }

    /// Add serial ordering edges from every task within `index`'s dependency
    /// subtree (including `index` itself) to the previous serial dependency, so
    /// that the whole subtree runs after it. Only `RunTask` nodes are ordered.
    ///
    /// Non-persistent tasks are ordered against `previous_standard` — the
    /// previous dependency that actually completes — instead of `previous`,
    /// as a persistent dependency never completes and so cannot order them.
    ///
    /// The walk only follows real dependency edges that block a node from
    /// running: never serial edges (following one would escape `index`'s real
    /// subtree), present dependency edges (which never took part in the serial
    /// order), `cleanup` edges (which point back to the task that owns the
    /// cleanup), or `wait` edges (which don't run to completion first).
    fn link_serial_required_edges(
        &mut self,
        index: NodeIndex,
        previous: Option<NodeIndex>,
        previous_standard: Option<NodeIndex>,
    ) {
        if previous.is_none() && previous_standard.is_none() {
            return;
        }

        let mut visited = FxHashSet::default();
        let mut ordered = vec![];
        let mut stack = vec![index];

        // Collect the subtree first, then link, as linking adds edges that
        // must not pollute the walk. `ordered` keeps the link order stable
        while let Some(node_index) = stack.pop() {
            if !visited.insert(node_index) {
                continue;
            }

            ordered.push(node_index);

            let mut children = self.graph.children(node_index);

            while let Some((edge_index, child_index)) = children.walk_next(&self.graph) {
                if !self.serial_edges.contains(&edge_index)
                    && !self.present_edges.contains(&edge_index)
                    && self
                        .graph
                        .edge_weight(edge_index)
                        .is_some_and(|weight| weight.is_required_type())
                    && matches!(self.graph[child_index], ActionNode::RunTask(_))
                {
                    stack.push(child_index);
                }
            }
        }

        for node_index in ordered {
            // A persistent action is marked as completed the moment it is
            // dispatched, so a non-persistent action ordered after one would
            // simply run immediately. Order those against the previous
            // dependency that does complete
            let anchor = if self.graph[node_index].is_persistent() {
                previous
            } else {
                previous_standard
            };

            if let Some(anchor) = anchor
                && anchor != node_index
                && let Some(edge_index) = self.try_link_edge(node_index, anchor)
            {
                self.serial_edges.insert(edge_index);
            }
        }
    }
}
