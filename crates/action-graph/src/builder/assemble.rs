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
use moon_pdk_api::{DefineRequirementsInput, RequirementPhase, ToolchainRequirement};
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
/// the toolchain when it's invalid, and resolve it to how the toolchain was
/// configured (stable or unstable).
fn parse_requirement_id(
    app_context: &AppContext,
    spec: &ToolchainSpec,
    require_id: String,
) -> miette::Result<Id> {
    let id =
        Id::new(&require_id).map_err(|error| ActionGraphError::InvalidToolchainRequirement {
            id: spec.id.to_string(),
            dep_id: require_id,
            error: Box::new(error),
        })?;

    Ok(app_context.toolchain_registry.resolve_toolchain_id(id))
}

/// What an action of a toolchain resolved to. Either the toolchain's own node,
/// or, when the toolchain doesn't have the action (or it's disabled), the nodes
/// that it falls through to: the toolchain's next action (install dependencies,
/// then setup environment, then setup toolchain), and the same action of the
/// toolchains that it requires before its own, so that the actions depending
/// on it are still ordered after them. A fall-through never includes the
/// workspace sync, which an action links to when it depends on nothing else.
#[derive(Clone, Debug)]
enum Slot {
    Node(NodeIndex),
    FallThrough(Vec<NodeIndex>),
}

impl Default for Slot {
    fn default() -> Self {
        Self::FallThrough(vec![])
    }
}

impl Slot {
    /// Fall through to the toolchain's next action, and the same action of the
    /// toolchains that it requires before its own.
    fn fall_through(next: &Slot, required_before: Vec<NodeIndex>) -> Self {
        let mut indices = next.indices().to_vec();

        for index in required_before {
            if !indices.contains(&index) {
                indices.push(index);
            }
        }

        Self::FallThrough(indices)
    }

    fn node(&self) -> Option<NodeIndex> {
        match self {
            Self::Node(index) => Some(*index),
            Self::FallThrough(_) => None,
        }
    }

    fn indices(&self) -> &[NodeIndex] {
        match self {
            Self::Node(index) => std::slice::from_ref(index),
            Self::FallThrough(indices) => indices,
        }
    }
}

/// What a toolchain's install slot resolved to: the install dependencies slot
/// in its root (or its setup, without a root), and the setup of the requested
/// version, when the root's actions run with another version.
#[derive(Clone, Debug, Default)]
struct InstallSlot {
    slot: Slot,
    own_toolchain: Slot,
}

impl InstallSlot {
    fn new(slot: Slot) -> Self {
        Self {
            slot,
            own_toolchain: Slot::default(),
        }
    }

    fn indices(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.slot
            .indices()
            .iter()
            .chain(self.own_toolchain.indices())
            .copied()
    }
}

/// The dependencies root that a toolchain's actions run in. It's the root
/// the toolchain was placed in, or for a required toolchain, the root of the
/// toolchain that requires it, as required toolchains (like a package manager
/// for JavaScript) typically don't locate a root themselves.
#[derive(Clone, Copy)]
struct RootScope<'a> {
    path: &'a WorkspaceRelativePathBuf,
    kind: &'a RootKind,
}

impl<'a> RootScope<'a> {
    fn new(root: &'a ToolchainDependenciesRoot) -> Self {
        Self {
            path: &root.root,
            kind: &root.kind,
        }
    }

    /// Whether the project is only the owner of a dependencies workspace,
    /// which isn't passed to the plugin, as the actions are for the whole
    /// workspace.
    fn owner_only(&self) -> bool {
        matches!(self.kind, RootKind::Workspace { .. })
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
    setup_toolchain_indexes: FxHashMap<(ToolchainSpec, Option<Id>), Slot>,

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
                        .await?
                        .into_iter()
                        .map(Some),
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
        let slot = match request {
            ProvisionRequest::InstallDependencies(spec, project) => {
                return Ok(self
                    .install_dependencies(&spec, Some(&project))
                    .await?
                    .indices()
                    .collect());
            }
            ProvisionRequest::InstallDependenciesByProject(project) => {
                return self
                    .install_dependencies_by_toolchains(&project, &project.toolchains)
                    .await;
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
                    RootScope {
                        path: &root,
                        kind: &RootKind::Package,
                    },
                    Some(&project),
                    FxHashSet::default(),
                ))
                .await?
            }
            ProvisionRequest::SetupProto => {
                Slot::FallThrough(self.setup_proto().into_iter().collect())
            }
            ProvisionRequest::SetupToolchain(spec, project) => {
                self.setup_toolchain(&spec, project.as_deref()).await?
            }
            ProvisionRequest::SetupToolchainAndEnvironment(spec) => {
                let slot = self.setup_toolchain(&spec, None).await?;

                if !slot.indices().is_empty() {
                    self.setup_environment_root(&spec).await?;
                }

                slot
            }
            ProvisionRequest::SetupEnvironmentRoot(spec) => {
                self.setup_environment_root(&spec).await?
            }
            ProvisionRequest::SyncWorkspace => {
                Slot::FallThrough(self.sync_workspace().into_iter().collect())
            }
        };

        Ok(slot.indices().to_vec())
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
    ) -> miette::Result<Slot> {
        let key = (spec.to_owned(), project.map(|project| project.id.clone()));

        if let Some(slot) = self.setup_toolchain_indexes.get(&key) {
            return Ok(slot.clone());
        }

        let slot =
            Box::pin(self.internal_setup_toolchain(spec, project, FxHashSet::default())).await?;

        self.setup_toolchain_indexes.insert(key, slot.clone());

        Ok(slot)
    }

    /// Setup a toolchain, which only inserts an action for a tier 3 toolchain,
    /// otherwise it falls through to the setup of the toolchains it requires
    /// before it, or the workspace sync (by its dependents).
    async fn internal_setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
        mut cycle: FxHashSet<Id>,
    ) -> miette::Result<Slot> {
        if spec.is_system() || cycle.contains(&spec.id) {
            return Ok(Slot::default());
        }

        let app_context = self.app_context;
        let mut required_before = vec![];
        let mut required_after = vec![];

        cycle.insert(spec.id.clone());

        // Toolchain may depend on others
        for (phase, require_spec) in self
            .get_required_specs(spec, project, |req| req.setup_toolchain)
            .await?
        {
            if require_spec.id == spec.id || cycle.contains(&require_spec.id) {
                continue;
            }

            match phase {
                RequirementPhase::Before => {
                    let slot = Box::pin(self.internal_setup_toolchain(
                        &require_spec,
                        project,
                        cycle.clone(),
                    ))
                    .await?;

                    required_before.extend_from_slice(slot.indices());
                }
                RequirementPhase::After => {
                    required_after.push(require_spec);
                }
            };
        }

        // Toolchain does not support tier 3 (or is explicitly disabled), so
        // there's nothing to setup, nor to setup others after
        if !self.has_setup_toolchain(spec).await? {
            return Ok(Slot::fall_through(&Slot::default(), required_before));
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
        self.try_link_edges(index, required_before);
        self.link_available_edges(index, edges)?;

        // Required toolchains that are setup after this one, like Node.js
        // after JavaScript, so a requirement that would close a cycle is skipped.
        // One that falls through has no action to order
        for require_spec in required_after {
            if !self.has_setup_toolchain(&require_spec).await? {
                continue;
            }

            if let Some(require_index) =
                Box::pin(self.internal_setup_toolchain(&require_spec, project, cycle.clone()))
                    .await?
                    .node()
            {
                self.try_link_edge(require_index, index);
            }
        }

        Ok(Slot::Node(index))
    }

    /// Setup the environment of a toolchain in a root, resolved with the
    /// project's config and versions, which only inserts an action when the
    /// toolchain implements `setup_environment`, otherwise it falls through to
    /// the toolchain's setup, and the environments of the toolchains it requires
    /// before it.
    async fn internal_setup_environment(
        &mut self,
        spec: &ToolchainSpec,
        root: RootScope<'_>,
        project: Option<&Project>,
        mut cycle: FxHashSet<Id>,
    ) -> miette::Result<Slot> {
        if spec.is_system() || cycle.contains(&spec.id) {
            return Ok(Slot::default());
        }

        let mut required_before = vec![];
        let mut required_after = vec![];

        cycle.insert(spec.id.clone());

        // Toolchain may depend on others. Required toolchains (like a package
        // manager for JavaScript) typically don't locate a dependencies root, as
        // this toolchain does, so their environment is setup in this toolchain's
        // root. One that doesn't setup an environment falls through to its
        // toolchain, which must then be setup beforehand (like uv for Python)
        for (phase, require_spec) in self
            .get_required_specs(spec, project, |req| req.setup_environment)
            .await?
        {
            // Skip if itself or already in cycle
            if require_spec.id == spec.id || cycle.contains(&require_spec.id) {
                continue;
            }

            match phase {
                RequirementPhase::Before => {
                    let slot = Box::pin(self.internal_setup_environment(
                        &require_spec,
                        root,
                        project,
                        cycle.clone(),
                    ))
                    .await?;

                    required_before.extend_from_slice(slot.indices());
                }
                RequirementPhase::After => {
                    required_after.push(require_spec);
                }
            };
        }

        let toolchain_slot = self.setup_toolchain(spec, project).await?;

        // Toolchain does not support it (or is explicitly disabled), so there's
        // nothing to setup, nor to setup others after
        if !self.has_setup_environment(spec).await? {
            return Ok(Slot::fall_through(&toolchain_slot, required_before));
        }

        // The node may already exist, but the edges must still be linked, as the
        // version is not part of its identity, so another version of the toolchain
        // (overridden by a project that shares the root) must also be setup first
        let sync_workspace_index = self.sync_workspace();
        let (project_id, owner_project_id) = split_owner_project(project, root.owner_only());
        let index = self.get_or_insert_node(
            NodeKey::SetupEnvironment {
                toolchain: spec.id.clone(),
                root: root.path.clone(),
            },
            || {
                ActionNode::setup_environment(SetupEnvironmentNode {
                    owner_project_id,
                    project_id,
                    root: root.path.clone(),
                    toolchain_id: spec.id.clone(),
                })
            },
        );

        self.link_required_edges(index, toolchain_slot.indices().to_vec())?;

        // Required environments that are setup before this one, which may
        // require this toolchain in turn (like Python and uv), when this
        // environment was already setup, so a requirement that would close
        // a cycle is skipped
        self.try_link_edges(index, required_before);
        self.link_sync_workspace_if_unlinked(index, sync_workspace_index)?;

        // Required environments that are setup after this one, in this root,
        // like Node.js's after npm's, so a requirement that would close a cycle
        // is skipped. One that falls through has no action to order
        for require_spec in required_after {
            if !self.has_setup_environment(&require_spec).await? {
                continue;
            }

            if let Some(env_index) = Box::pin(self.internal_setup_environment(
                &require_spec,
                root,
                project,
                cycle.clone(),
            ))
            .await?
            .node()
            {
                self.try_link_edge(env_index, index);
            }
        }

        Ok(Slot::Node(index))
    }

    /// Return the toolchains that a toolchain requires for an action, with
    /// the phase they run in, relative to the toolchain's own action. Errors
    /// when a required toolchain is not configured.
    async fn get_required_specs(
        &self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
        get_phase: impl Fn(&ToolchainRequirement) -> Option<RequirementPhase>,
    ) -> miette::Result<Vec<(RequirementPhase, ToolchainSpec)>> {
        let app_context = self.app_context;
        let toolchain_registry = &app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;
        let mut specs = vec![];

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

        for requirement in output.requires {
            let Some(phase) = get_phase(&requirement) else {
                continue;
            };

            let require_id = parse_requirement_id(app_context, spec, requirement.id)?;

            let Some(require_spec) = get_spec(app_context, &require_id, project) else {
                return Err(ActionGraphError::MissingToolchainRequirement {
                    id: spec.id.to_string(),
                    dep_id: require_id.to_string(),
                }
                .into());
            };

            specs.push((phase, require_spec));
        }

        Ok(specs)
    }

    /// Setup the environment in the workspace root, without an associated
    /// project. This inserts no actions at all, unless the located
    /// dependencies root is the workspace root.
    async fn setup_environment_root(&mut self, spec: &ToolchainSpec) -> miette::Result<Slot> {
        // Explicitly disabled
        if !self.options.setup_environment.is_enabled(&spec.id) || spec.is_system() {
            return Ok(Slot::default());
        }

        // Only insert actions if the dependencies root is the workspace root
        let Some(root) = self.get_workspace_root(spec) else {
            return Ok(Slot::default());
        };

        let (root_spec, owner) = self.get_root_chain(root, spec, None)?;

        Box::pin(self.internal_setup_environment(
            &root_spec,
            RootScope::new(root),
            owner.as_deref(),
            FxHashSet::default(),
        ))
        .await
    }

    async fn install_dependencies_by_toolchains(
        &mut self,
        project: &Project,
        toolchains: &[Id],
    ) -> miette::Result<Vec<NodeIndex>> {
        let mut indices = vec![];

        for toolchain_id in toolchains {
            if let Some(spec) = get_project_spec(self.app_context, toolchain_id, project) {
                indices.extend(
                    self.install_dependencies(&spec, Some(project))
                        .await?
                        .indices(),
                );
            }
        }

        Ok(indices)
    }

    /// Resolve the "install slot" of a toolchain: install its dependencies in
    /// the closest root, or the most specific action that exists.
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
            return Ok(slot.clone());
        }

        let slot = self.internal_install_dependencies(spec, project).await?;

        self.install_deps_indexes.insert(key, slot.clone());

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
        if !self.installs_dependencies(&spec.id) {
            return Ok(InstallSlot::new(self.setup_toolchain(spec, project).await?));
        }

        // The workspace is synced before installing, even when there's
        // nothing to install
        self.sync_workspace();

        let layout = self.layout;

        let binding = match project {
            Some(project) => layout.binding(&project.id, &spec.id),
            None => layout.workspace_binding(&spec.id),
        };

        // A toolchain without tier 2, without a located root, or that isn't a
        // member of it, has no dependencies to install nor environment to
        // setup, so it falls through to its toolchain's setup
        let Some(Placement::Root { index, .. }) = binding.map(|binding| &binding.placement) else {
            return Ok(InstallSlot::new(
                Box::pin(self.install_dependencies_in(spec, None, project, FxHashSet::default()))
                    .await?,
            ));
        };

        // The actions are scoped to the root, so that every project placed in
        // it collapses into the same actions, which run with the root's spec
        // (its owner project's, otherwise the workspace's)
        let root = layout.root(*index);
        let (root_spec, owner) = self.get_root_chain(root, spec, project)?;
        let chain_project = owner.as_deref().or(match root.kind {
            RootKind::Package => project,
            RootKind::Workspace { .. } => None,
        });

        let slot = Box::pin(self.install_dependencies_in(
            &root_spec,
            Some(RootScope::new(root)),
            chain_project,
            FxHashSet::default(),
        ))
        .await?;

        // A project that overrides the root's version only needs its own
        // version to run its tasks, so it's linked by the requester instead
        let own_toolchain = if *spec == root_spec {
            Slot::default()
        } else {
            self.setup_toolchain(spec, project).await?
        };

        Ok(InstallSlot {
            slot,
            own_toolchain,
        })
    }

    /// Install the dependencies of a toolchain in a root, which only inserts an
    /// action when the toolchain implements `install_dependencies`, otherwise it
    /// falls through to the toolchain's environment (or its setup, without a
    /// root), and the installs of the toolchains it requires before it.
    async fn install_dependencies_in(
        &mut self,
        spec: &ToolchainSpec,
        root: Option<RootScope<'_>>,
        project: Option<&Project>,
        mut cycle: FxHashSet<Id>,
    ) -> miette::Result<Slot> {
        if spec.is_system() || cycle.contains(&spec.id) {
            return Ok(Slot::default());
        }

        // Only the toolchain is setup when installing its dependencies is
        // disabled, like a toolchain's own install slot
        if !self.installs_dependencies(&spec.id) {
            return self.setup_toolchain(spec, project).await;
        }

        let toolchain_slot = self.setup_toolchain(spec, project).await?;
        let env_slot = match root {
            Some(root) => {
                Box::pin(self.internal_setup_environment(spec, root, project, FxHashSet::default()))
                    .await?
            }
            None => toolchain_slot,
        };

        let mut required_before = vec![];
        let mut required_after = vec![];

        cycle.insert(spec.id.clone());

        // Toolchain may depend on others, like a package manager that installs
        // after its runtime does, in this toolchain's root
        for (phase, require_spec) in self
            .get_required_specs(spec, project, |req| req.install_dependencies)
            .await?
        {
            if require_spec.id == spec.id || cycle.contains(&require_spec.id) {
                continue;
            }

            match phase {
                RequirementPhase::Before => {
                    let slot = Box::pin(self.install_dependencies_in(
                        &require_spec,
                        root,
                        project,
                        cycle.clone(),
                    ))
                    .await?;

                    required_before.extend_from_slice(slot.indices());
                }
                RequirementPhase::After => {
                    required_after.push(require_spec);
                }
            };
        }

        // Toolchain does not support it, or there's no root to install in, so
        // there's nothing to install, nor to install others after
        let Some(root) = root else {
            return Ok(Slot::fall_through(&env_slot, required_before));
        };

        if !self.has_install_dependencies(spec).await? {
            return Ok(Slot::fall_through(&env_slot, required_before));
        }

        let sync_workspace_index = self.sync_workspace();
        let (project_id, owner_project_id) = split_owner_project(project, root.owner_only());
        let index = self.get_or_insert_node(
            NodeKey::InstallDependencies {
                toolchain: spec.id.clone(),
                root: root.path.clone(),
            },
            || {
                ActionNode::install_dependencies(InstallDependenciesNode {
                    members: match root.kind {
                        RootKind::Workspace { members } => Some(members.to_owned()),
                        RootKind::Package => None,
                    },
                    owner_project_id,
                    project_id,
                    root: root.path.clone(),
                    toolchain_id: spec.id.clone(),
                })
            },
        );

        self.link_required_edges(index, env_slot.indices().to_vec())?;
        self.try_link_edges(index, required_before);
        self.link_sync_workspace_if_unlinked(index, sync_workspace_index)?;

        // Required installs that run after this one, in this root, so a
        // requirement that would close a cycle is skipped. One that falls
        // through has no action to order
        for require_spec in required_after {
            if !self.has_install_dependencies(&require_spec).await? {
                continue;
            }

            if let Some(require_index) = Box::pin(self.install_dependencies_in(
                &require_spec,
                Some(root),
                project,
                cycle.clone(),
            ))
            .await?
            .node()
            {
                self.try_link_edge(require_index, index);
            }
        }

        Ok(Slot::Node(index))
    }

    /// Install dependencies in the workspace root, without an associated
    /// project, when the located dependencies root is the workspace root.
    async fn install_dependencies_root(
        &mut self,
        spec: &ToolchainSpec,
    ) -> miette::Result<InstallSlot> {
        // Explicitly disabled
        if spec.is_system() || !self.installs_dependencies(&spec.id) {
            return Ok(InstallSlot::default());
        }

        // Only insert actions if the dependencies root is the workspace root
        if self.get_workspace_root(spec).is_none() {
            return Ok(InstallSlot::default());
        }

        self.install_dependencies(spec, None).await
    }

    /// Whether the toolchain has its own setup toolchain action: it's tier 3,
    /// and setting it up is enabled.
    async fn has_setup_toolchain(&self, spec: &ToolchainSpec) -> miette::Result<bool> {
        Ok(!spec.is_system()
            && self.options.setup_toolchains.is_enabled(&spec.id)
            && self
                .app_context
                .toolchain_registry
                .load(&spec.id)
                .await?
                .supports_tier_3()
                .await)
    }

    /// Whether the toolchain has its own setup environment action (in a
    /// root): it implements `setup_environment`, and setting it up is enabled.
    async fn has_setup_environment(&self, spec: &ToolchainSpec) -> miette::Result<bool> {
        Ok(!spec.is_system()
            && self.options.setup_environment.is_enabled(&spec.id)
            && self
                .app_context
                .toolchain_registry
                .load(&spec.id)
                .await?
                .has_func("setup_environment")
                .await)
    }

    /// Whether the toolchain has its own install dependencies action (in a
    /// root): it implements `install_dependencies`, and installing is enabled.
    async fn has_install_dependencies(&self, spec: &ToolchainSpec) -> miette::Result<bool> {
        Ok(!spec.is_system()
            && self.installs_dependencies(&spec.id)
            && self
                .app_context
                .toolchain_registry
                .load(&spec.id)
                .await?
                .has_func("install_dependencies")
                .await)
    }

    /// Whether installing dependencies is enabled for the toolchain, by the
    /// builder's options and the toolchain's config.
    fn installs_dependencies(&self, id: &Id) -> bool {
        self.options.install_dependencies.is_enabled(id)
            && self
                .app_context
                .toolchains_config
                .get_plugin_config(id)
                .is_none_or(|cfg| cfg.install_dependencies)
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

    /// The spec that a root's actions run with, and the owner project that
    /// they resolve config and versions with, which is a package's owner when
    /// it isn't the requesting project, or a workspace's root project. A
    /// workspace without a root project resolves with the workspace's.
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
            (RootKind::Workspace { .. }, Some(owner_id)) => {
                Some(self.workspace_graph.get_project(owner_id)?)
            }
            _ => None,
        };

        // The root's toolchain is always the requested toolchain
        debug_assert_eq!(root.spec.id, spec.id);

        Ok((root.spec.clone(), owner))
    }

    // EDGES

    /// Link an action that depends on no other action to the workspace sync,
    /// as every action runs after the workspace has been synced.
    fn link_sync_workspace_if_unlinked(
        &mut self,
        index: NodeIndex,
        sync_workspace_index: Option<NodeIndex>,
    ) -> miette::Result<()> {
        if self
            .graph
            .edges_directed(index, Direction::Outgoing)
            .next()
            .is_none()
        {
            self.link_available_edges(index, vec![sync_workspace_index])?;
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

/// The project to pass to a root's plugin, and the owner project to resolve
/// its config with instead, when it's only the owner of a dependencies
/// workspace.
fn split_owner_project(project: Option<&Project>, owner_only: bool) -> (Option<Id>, Option<Id>) {
    let id = project.map(|project| project.id.clone());

    if owner_only { (None, id) } else { (id, None) }
}
