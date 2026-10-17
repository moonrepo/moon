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
use crate::action_graph::{ActionGraph, ActionGraphType};
use crate::action_graph_error::ActionGraphError;
use crate::action_graph_options::ActionGraphBuilderOptions;
use moon_action::{
    ActionNode, InstallDependenciesNode, SetupEnvironmentNode, SetupToolchainNode, SyncProjectNode,
};
use moon_app_context::AppContext;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, color};
use moon_config::{TaskDependencyConfig, TaskDependencyType};
use moon_graph_utils::{CycleSpace, add_acyclic_edge, add_acyclic_edges};
use moon_layout_graph::{LayoutGraph, Placement, RootKind, ToolchainDependenciesRoot};
use moon_pdk_api::DefineRequirementsInput;
use moon_project::Project;
use moon_task::Target;
use moon_toolchain::ToolchainSpec;
use moon_workspace_graph::WorkspaceGraph;
use moon_workspace_graph::projects::ScopePartition;
use petgraph::prelude::*;
use petgraph::visit::{VisitMap, Visitable};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use tracing::{debug, trace, warn};

/// The identity of a provisioning or sync node. Task nodes are identified by
/// their instance, as instances are already unique.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum NodeKey {
    // A root's members and project are derived from the root, so they're
    // not part of its identity
    InstallDependencies {
        toolchain: Id,
        root: WorkspaceRelativePathBuf,
    },
    SetupEnvironment {
        toolchain: Id,
        root: WorkspaceRelativePathBuf,
    },
    SetupProto,
    SetupToolchain(ToolchainSpec),
    SyncProject(Id),
    SyncWorkspace,
}

/// Parse the ID of a toolchain that a toolchain's plugin requires, naming
/// the toolchain when it's invalid.
fn parse_requirement_id(spec: &ToolchainSpec, require_id: String) -> miette::Result<Id> {
    Id::new(&require_id).map_err(|error| {
        ActionGraphError::InvalidToolchainRequirement {
            id: spec.id.to_string(),
            dep_id: require_id,
            error: Box::new(error),
        }
        .into()
    })
}

/// What a toolchain's install slot resolved to: the most specific node that
/// exists for its root (or its setup, without a root), and the setup of the
/// requested version, when the root's actions run with another version.
#[derive(Clone, Copy, Debug, Default)]
struct InstallSlot {
    index: Option<NodeIndex>,
    own_toolchain: Option<NodeIndex>,
}

impl InstallSlot {
    fn new(index: Option<NodeIndex>) -> Self {
        Self {
            index,
            own_toolchain: None,
        }
    }

    fn indices(self) -> impl Iterator<Item = NodeIndex> {
        self.index.into_iter().chain(self.own_toolchain)
    }
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
    layout: &'app LayoutGraph,
    options: &'app ActionGraphBuilderOptions,
    workspace_graph: &'app WorkspaceGraph,

    graph: ActionGraphType,
    cycle_space: CycleSpace,
    keys: FxHashMap<NodeKey, NodeIndex>,
    cleanup_indices: FxHashSet<NodeIndex>,

    instance_nodes: Vec<Option<NodeIndex>>,
    request_nodes: FxHashMap<RequestId, Vec<NodeIndex>>,

    /// Install deps and toolchain setups already resolved for a toolchain
    /// and project (by ID). Resolving one again would return the same node,
    /// and only link edges that already exist (or that would still close a
    /// cycle), but would call the toolchain's plugin functions again. Keyed by
    /// project ID, as every request for a project passes the same project (the
    /// workspace graph's), and its config decides the result.
    install_deps_indexes: FxHashMap<(ToolchainSpec, Option<Id>), InstallSlot>,
    setup_toolchain_indexes: FxHashMap<(ToolchainSpec, Option<Id>), Option<NodeIndex>>,

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
    layout: &LayoutGraph,
    plan: &mut TaskPlan,
) -> miette::Result<AssembledGraph> {
    let mut instances = std::mem::take(&mut plan.instances)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();

    let mut assembler = GraphAssembler {
        app_context,
        layout,
        options,
        workspace_graph,
        graph: ActionGraphType::new(),
        cycle_space: CycleSpace::default(),
        keys: FxHashMap::default(),
        cleanup_indices: FxHashSet::default(),
        instance_nodes: vec![None; instances.len()],
        request_nodes: FxHashMap::default(),
        install_deps_indexes: FxHashMap::default(),
        setup_toolchain_indexes: FxHashMap::default(),
        task_edges: FxHashMap::default(),
        present_edges: FxHashSet::default(),
        serial_edges: FxHashSet::default(),
    };

    for event in std::mem::take(&mut plan.events) {
        assembler.replay(event, &mut instances).await?;
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

impl<'app> GraphAssembler<'app> {
    // EVENTS

    async fn replay(
        &mut self,
        event: Event,
        instances: &mut [Option<ActionNode>],
    ) -> miette::Result<()> {
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
                task,
                sync_project,
            } => {
                let mut edges = vec![sync_project.map(|id| self.keys[&NodeKey::SyncProject(id)])];

                edges.extend(
                    self.install_dependencies_by_toolchains(&project, &task.toolchains)
                        .await?,
                );

                // If no edges created, we should at minimum sync the workspace
                if edges.iter().all(|edge| edge.is_none()) {
                    edges.push(self.sync_workspace());
                }

                let node = instances[instance]
                    .take()
                    .expect("Task instance has already been inserted.");
                let index = self.insert_node(None, node);

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

                // So far, the task only links to setup and sync actions, and
                // to tasks it's a cleanup of (with a cleanup edge, while
                // requirements never are), so a requirement was only linked
                // already when it's listed more than once, which is checked
                // without scanning the task's edges
                let dedupe = requirements.len() > 1;
                let mut linked = FxHashSet::default();

                for (dep_instance, weight) in requirements {
                    debug_assert!(
                        weight != TaskDependencyType::Cleanup,
                        "Cleanups are linked separately."
                    );

                    if dedupe && !linked.insert((dep_instance, weight)) {
                        continue;
                    }

                    self.link_edge(index, self.get_instance_node(dep_instance), weight)?;
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
                return Ok(self
                    .install_dependencies(&spec, Some(&project))
                    .await?
                    .indices()
                    .collect());
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
                return Ok(self
                    .install_dependencies_root(&spec)
                    .await?
                    .indices()
                    .collect());
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
        ActionGraph::new_with_cleanups(self.graph, self.cleanup_indices)
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
        let key = (spec.to_owned(), project.map(|project| project.id.clone()));

        if let Some(index) = self.setup_toolchain_indexes.get(&key) {
            return Ok(*index);
        }

        let index =
            Box::pin(self.internal_setup_toolchain(spec, project, FxHashSet::default())).await?;

        self.setup_toolchain_indexes.insert(key, index);

        Ok(index)
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
                let require_id = parse_requirement_id(spec, require_id)?;

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
            let require_id = parse_requirement_id(spec, require_id)?;

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
        let Some(root) = self.get_workspace_root(spec) else {
            return Ok(None);
        };

        let (root_spec, owner) = self.get_root_chain(root, spec, None)?;

        Box::pin(self.internal_setup_environment(
            &root_spec,
            &root.root,
            owner.as_deref(),
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
                let slot = self.install_dependencies(&spec, Some(project)).await?;

                indices.push(slot.index);
                indices.push(slot.own_toolchain);
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
    ) -> miette::Result<InstallSlot> {
        if spec.is_system() {
            return Ok(InstallSlot::default());
        }

        let key = (spec.to_owned(), project.map(|project| project.id.clone()));

        if let Some(slot) = self.install_deps_indexes.get(&key) {
            return Ok(*slot);
        }

        let slot = self.internal_install_dependencies(spec, project).await?;

        self.install_deps_indexes.insert(key, slot);

        Ok(slot)
    }

    async fn internal_install_dependencies(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> miette::Result<InstallSlot> {
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
            return Ok(InstallSlot::new(self.setup_toolchain(spec, project).await?));
        }

        let sync_workspace_index = self.sync_workspace();
        let layout = self.layout;

        let binding = match project {
            Some(project) => layout.binding(&project.id, &spec.id),
            None => layout.workspace_binding(&spec.id),
        };

        // Only insert this action if the project was placed in a root, as a
        // toolchain without tier 2, without a located root, or that isn't a
        // member of it, only sets up the toolchain
        let Some(Placement::Root { index, .. }) = binding.map(|binding| &binding.placement) else {
            return Ok(InstallSlot::new(self.setup_toolchain(spec, project).await?));
        };

        // The actions are scoped to the root, so that every project placed in
        // it collapses into the same actions, which run with the root's spec
        // (the workspace's, or its owner project's for a package)
        let root = layout.root(*index);
        let (root_spec, owner) = self.get_root_chain(root, spec, project)?;
        let chain_project = owner.as_deref().or(match root.kind {
            RootKind::Package => project,
            RootKind::Workspace { .. } => None,
        });

        let setup_toolchain_index = self.setup_toolchain(&root_spec, chain_project).await?;
        let setup_env_index = Box::pin(self.internal_setup_environment(
            &root_spec,
            &root.root,
            chain_project,
            FxHashSet::default(),
        ))
        .await?;

        let index = if root.capabilities.install_dependencies {
            let index = self.get_or_insert_node(
                NodeKey::InstallDependencies {
                    toolchain: spec.id.clone(),
                    root: root.root.clone(),
                },
                || {
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: match &root.kind {
                            RootKind::Workspace { members } => Some(members.to_owned()),
                            RootKind::Package => None,
                        },
                        project_id: chain_project.map(|project| project.id.clone()),
                        root: root.root.clone(),
                        toolchain_id: spec.id.clone(),
                    })
                },
            );

            self.link_first_available_edge(
                index,
                vec![setup_env_index, setup_toolchain_index, sync_workspace_index],
            )?;

            Some(index)
        }
        // Otherwise pass through to setup environment
        else if let Some(setup_env_index) = setup_env_index {
            self.link_first_available_edge(
                setup_env_index,
                vec![setup_toolchain_index, sync_workspace_index],
            )?;

            Some(setup_env_index)
        }
        // Or fallback entirely to setup toolchain
        else {
            setup_toolchain_index
        };

        // A project that overrides the root's version only needs its own
        // version to run its tasks, so it's linked by the requester instead
        let own_toolchain = if *spec == root_spec {
            None
        } else {
            self.setup_toolchain(spec, project).await?
        };

        Ok(InstallSlot {
            index,
            own_toolchain,
        })
    }

    /// Install dependencies in the workspace root, without an associated
    /// project, when the located dependencies root is the workspace root.
    async fn install_dependencies_root(
        &mut self,
        spec: &ToolchainSpec,
    ) -> miette::Result<InstallSlot> {
        // Explicitly disabled
        if spec.is_system()
            || !self.options.install_dependencies.is_enabled(&spec.id)
            || self
                .app_context
                .toolchains_config
                .get_plugin_config(&spec.id)
                .is_some_and(|cfg| !cfg.install_dependencies)
        {
            return Ok(InstallSlot::default());
        }

        // Only insert actions if the dependencies root is the workspace root
        if self.get_workspace_root(spec).is_none() {
            return Ok(InstallSlot::default());
        }

        self.install_dependencies(spec, None).await
    }

    /// The root that the workspace was placed in for the toolchain, when the
    /// root is the workspace root.
    fn get_workspace_root(&self, spec: &ToolchainSpec) -> Option<&'app ToolchainDependenciesRoot> {
        let layout = self.layout;

        layout
            .workspace_binding(&spec.id)
            .and_then(|binding| binding.root_index())
            .map(|index| layout.root(index))
            .filter(|root| root.root.as_str().is_empty())
    }

    /// The spec that a root's actions run with, and the owner project that its
    /// actions are associated with, which is only a package's owner when it
    /// isn't the requesting project. A workspace's actions have no project,
    /// so that its requirements resolve with the workspace's config.
    fn get_root_chain(
        &self,
        root: &ToolchainDependenciesRoot,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> miette::Result<(ToolchainSpec, Option<Arc<Project>>)> {
        let owner = match (&root.kind, &root.owner_project) {
            (RootKind::Package, Some(owner_id))
                if project.is_none_or(|project| &project.id != owner_id) =>
            {
                Some(self.workspace_graph.get_project(owner_id)?)
            }
            _ => None,
        };

        // The root's toolchain is always the requested toolchain
        debug_assert_eq!(root.spec.id, spec.id);

        Ok((root.spec.clone(), owner))
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
        if add_acyclic_edge(&mut self.graph, from, to, weight, &mut self.cycle_space).is_err() {
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

        match add_acyclic_edge(
            &mut self.graph,
            from,
            to,
            TaskDependencyType::Required,
            &mut self.cycle_space,
        ) {
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
            .edges_connecting(from, to)
            .any(|edge| *edge.weight() == weight)
    }

    /// Describe the cycle that an edge from `from` to `to` would introduce,
    /// as `to` already reaches `from`.
    fn find_cycle(&self, from: NodeIndex, to: NodeIndex) -> Option<String> {
        let graph = &self.graph;
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
                && add_acyclic_edges(
                    &mut self.graph,
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

        for index in self.graph.node_indices() {
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

                        match add_acyclic_edge(
                            &mut self.graph,
                            *owner_index,
                            *dep_index,
                            weight,
                            &mut self.cycle_space,
                        ) {
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
        if links.is_empty() {
            return;
        }

        let children = self.collect_serial_children();

        for link in links {
            self.link_serial_required_edges(
                &children,
                self.get_instance_node(link.index),
                link.previous
                    .map(|instance| self.get_instance_node(instance)),
                link.previous_standard
                    .map(|instance| self.get_instance_node(instance)),
            );
        }
    }

    /// Collect the dependencies that a serial walk follows from each task: real
    /// dependency edges that block it from running, to another task. Never
    /// serial edges (following one would escape the real subtree), present
    /// dependency edges (which never took part in the serial order), `cleanup`
    /// edges (which point back to the task that owns the cleanup), or `wait`
    /// edges (which don't run to completion first).
    ///
    /// They're all linked before any serial edge is, and serial edges are never
    /// followed, so they're collected once, in the order the edges are walked,
    /// instead of walking each task's edges (serial ones included) for every
    /// link, as lower tasks gain a serial edge for every link above them.
    fn collect_serial_children(&self) -> Vec<Vec<NodeIndex>> {
        let mut children = vec![vec![]; self.graph.node_count()];

        for index in self.graph.node_indices() {
            if !matches!(self.graph[index], ActionNode::RunTask(_)) {
                continue;
            }

            for edge in self.graph.edges_directed(index, Direction::Outgoing) {
                if !self.present_edges.contains(&edge.id())
                    && edge.weight().is_required_type()
                    && matches!(self.graph[edge.target()], ActionNode::RunTask(_))
                {
                    children[index.index()].push(edge.target());
                }
            }
        }

        children
    }

    /// Add serial ordering edges from every task within `index`'s dependency
    /// subtree (including `index` itself) to the previous serial dependency, so
    /// that the whole subtree runs after it. Only `RunTask` nodes are ordered,
    /// and the subtree is walked through `children` (see
    /// [`GraphAssembler::collect_serial_children`]).
    ///
    /// Non-persistent tasks are ordered against `previous_standard` — the
    /// previous dependency that actually completes — instead of `previous`,
    /// as a persistent dependency never completes and so cannot order them.
    fn link_serial_required_edges(
        &mut self,
        children: &[Vec<NodeIndex>],
        index: NodeIndex,
        previous: Option<NodeIndex>,
        previous_standard: Option<NodeIndex>,
    ) {
        if previous.is_none() && previous_standard.is_none() {
            return;
        }

        let mut visited = FxHashSet::default();
        let mut links = vec![];
        let mut stack = vec![index];

        // Collect the subtree first, then link, as linking adds edges that
        // must not pollute the walk. `links` keeps the link order stable
        while let Some(node_index) = stack.pop() {
            if !visited.insert(node_index) {
                continue;
            }

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
            {
                links.push((node_index, anchor));
            }

            stack.extend(children[node_index.index()].iter().copied());
        }

        let Some(&(_, anchor)) = links.first() else {
            return;
        };

        // When linking to both anchors, an edge into one can change what the
        // other reaches, so each edge is checked for a cycle on its own
        if links.iter().any(|(_, other)| *other != anchor) {
            for (node_index, anchor) in links {
                if let Some(edge_index) = self.try_link_edge(node_index, anchor) {
                    self.serial_edges.insert(edge_index);
                }
            }

            return;
        }

        // Otherwise an edge into the anchor would close a cycle only when the
        // anchor already reaches the node, and adding an edge into the anchor
        // never changes what it reaches (only an edge from a node it reaches
        // could), so what it reaches is found once, instead of searching for
        // a path for every edge. Nodes that already require the anchor are
        // found once too, instead of scanning each node's edges
        let mut reachable = Dfs::new(&self.graph, anchor);

        while reachable.next(&self.graph).is_some() {}

        let mut requiring = self.graph.visit_map();

        for edge in self.graph.edges_directed(anchor, Direction::Incoming) {
            if *edge.weight() == TaskDependencyType::Required {
                requiring.visit(edge.source());
            }
        }

        for (node_index, anchor) in links {
            if requiring.is_visited(&node_index) {
                continue;
            }

            if reachable.discovered.is_visited(&node_index) {
                trace!(
                    index = node_index.index(),
                    requires = ?[anchor.index()],
                    "Not linking requirements for index, as they would introduce a cycle"
                );

                continue;
            }

            trace!(
                index = node_index.index(),
                requires = ?[anchor.index()],
                "Linking requirements for index"
            );

            let edge_index = self
                .graph
                .add_edge(node_index, anchor, TaskDependencyType::Required);

            self.serial_edges.insert(edge_index);
        }
    }
}
