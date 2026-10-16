// Each test binary only uses some of these utilities
#![allow(dead_code)]

use moon_action::ActionNode;
use moon_action_context::ActionContext;
use moon_action_graph::*;
use moon_affected::{Affected, AffectedProjectState, AffectedTaskState};
use moon_config::TaskDependencyType;
use moon_test_utils::WorkspaceMocker;
use moon_workspace_graph::WorkspaceGraph;
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use rustc_hash::FxHashSet;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ActionGraphContainer {
    pub mocker: WorkspaceMocker,
}

impl ActionGraphContainer {
    pub fn new(root: &Path) -> Self {
        Self {
            mocker: WorkspaceMocker::new(root)
                .load_default_configs()
                .with_all_toolchains()
                .with_test_toolchains()
                .with_default_projects()
                .with_global_envs(),
        }
    }

    pub fn set_working_dir(mut self, dir: PathBuf) -> Self {
        self.mocker = self.mocker.set_working_dir(dir);
        self
    }

    pub async fn create_workspace_graph(&self) -> Arc<WorkspaceGraph> {
        Arc::new(self.mocker.mock_workspace_graph().await)
    }

    pub async fn create_builder(
        &mut self,
        workspace_graph: Arc<WorkspaceGraph>,
    ) -> ActionGraphBuilder {
        let config = &self.mocker.workspace_config.pipeline;

        self.create_builder_with_options(
            workspace_graph,
            ActionGraphBuilderOptions {
                install_dependencies: config.install_dependencies.clone(),
                sync_projects: config.sync_projects.clone(),
                sync_workspace: config.sync_workspace,
                ..Default::default()
            },
        )
        .await
    }

    pub async fn create_builder_with_options(
        &mut self,
        workspace_graph: Arc<WorkspaceGraph>,
        options: ActionGraphBuilderOptions,
    ) -> ActionGraphBuilder {
        ActionGraphBuilder::new(
            Arc::new(self.mocker.mock_app_context()),
            workspace_graph,
            options,
        )
    }
}

/// The identity of a node, as a string. Unlike the label, it includes every
/// field that is part of the node's identity (`Hash` and `Eq`), as labels
/// collide for the dependency variants of a task (different args or env),
/// and for nodes that only differ by project or members. It never includes
/// a node index, so it's stable across insertion orders and builders.
pub fn get_node_key(node: &ActionNode) -> String {
    let mut key = match node {
        // The label masks the version in tests
        ActionNode::SetupProto(inner) => format!("SetupProto({})", inner.version),
        _ => node.label(),
    };

    match node {
        ActionNode::InstallDependencies(inner) => {
            if let Some(members) = &inner.members {
                key.push_str(&format!(" members={members:?}"));
            }

            if let Some(project_id) = &inner.project_id {
                key.push_str(&format!(" project={project_id}"));
            }
        }
        ActionNode::SetupEnvironment(inner) => {
            if let Some(project_id) = &inner.project_id {
                key.push_str(&format!(" project={project_id}"));
            }
        }
        ActionNode::RunTask(inner) => {
            if !inner.args.is_empty() {
                key.push_str(&format!(" args={:?}", inner.args));
            }

            if !inner.env.is_empty() {
                let mut env = inner.env.iter().collect::<Vec<_>>();
                env.sort_by(|a, b| a.0.cmp(b.0));

                key.push_str(&format!(" env={env:?}"));
            }
        }
        _ => {}
    };

    key
}

fn get_edge_weight(weight: &TaskDependencyType) -> &'static str {
    match weight {
        TaskDependencyType::Cleanup => "cleanup",
        TaskDependencyType::Optional => "optional",
        TaskDependencyType::Required => "required",
        TaskDependencyType::Wait => "wait",
    }
}

fn sorted<T: ToString>(items: impl IntoIterator<Item = T>) -> BTreeSet<String> {
    items.into_iter().map(|item| item.to_string()).collect()
}

/// A node's payload that isn't part of its identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalNode {
    pub label: String,
    pub id: u64,
    pub priority: u8,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct CanonicalAffectedState {
    pub dependencies: BTreeSet<String>,
    pub distance: u32,
    pub env: BTreeSet<String>,
    pub files: BTreeSet<String>,
    pub other: bool,
    pub tasks: BTreeSet<String>,
}

impl From<&AffectedProjectState> for CanonicalAffectedState {
    fn from(state: &AffectedProjectState) -> Self {
        Self {
            dependencies: sorted(&state.dependencies),
            distance: state.distance,
            files: sorted(&state.files),
            other: state.other,
            tasks: sorted(&state.tasks),
            ..Default::default()
        }
    }
}

impl From<&AffectedTaskState> for CanonicalAffectedState {
    fn from(state: &AffectedTaskState) -> Self {
        Self {
            dependencies: sorted(&state.dependencies),
            distance: state.distance,
            env: sorted(&state.env),
            files: sorted(&state.files),
            other: state.other,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct CanonicalAffected {
    pub projects: BTreeMap<String, CanonicalAffectedState>,
    pub should_check: bool,
    pub tasks: BTreeMap<String, CanonicalAffectedState>,
}

impl From<&Affected> for CanonicalAffected {
    fn from(affected: &Affected) -> Self {
        Self {
            projects: affected
                .projects
                .iter()
                .map(|(id, state)| (id.to_string(), state.into()))
                .collect(),
            should_check: affected.should_check,
            tasks: affected
                .tasks
                .iter()
                .map(|(target, state)| (target.to_string(), state.into()))
                .collect(),
        }
    }
}

/// Every data field of an `ActionContext`, sorted. The fields that are only
/// mutated while running (tokens, mutexes) are excluded, while target states
/// are included, as the builder sets the passthrough states.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct CanonicalActionContext {
    pub affected: Option<CanonicalAffected>,
    pub changed_files: BTreeSet<String>,
    pub cleanup_targets: BTreeSet<String>,
    pub ignored_dependencies: BTreeMap<String, BTreeSet<String>>,
    pub initial_targets: BTreeSet<String>,
    pub output_style: Option<String>,
    pub passthrough_args: Vec<String>,
    pub primary_targets: BTreeSet<String>,
    pub target_states: BTreeMap<String, String>,
    pub wait_targets: BTreeSet<String>,
}

impl From<&ActionContext> for CanonicalActionContext {
    fn from(context: &ActionContext) -> Self {
        Self {
            affected: context.affected.as_ref().map(CanonicalAffected::from),
            changed_files: sorted(&context.changed_files),
            cleanup_targets: sorted(&context.cleanup_targets),
            ignored_dependencies: context
                .ignored_dependencies
                .iter()
                .map(|(target, deps)| (target.to_string(), sorted(deps)))
                .collect(),
            initial_targets: sorted(
                context
                    .initial_targets
                    .iter()
                    .map(|locator| locator.as_str()),
            ),
            output_style: context
                .output_style
                .as_ref()
                .map(|style| format!("{style:?}")),
            passthrough_args: context.passthrough_args.clone(),
            primary_targets: sorted(&context.primary_targets),
            target_states: context
                .get_target_states()
                .into_iter()
                .map(|(target, state)| (target.to_string(), format!("{state:?}")))
                .collect(),
            wait_targets: sorted(&context.wait_targets),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct CanonicalPartition {
    /// The key of each partitioned node, and its target.
    pub targets: BTreeSet<(String, String)>,
    pub size: Option<usize>,
}

/// A built action graph and its context, in a form that doesn't depend on
/// node indices or insertion order, so that graphs built with different
/// requests, or by different builders, can be compared. Two builds are
/// equivalent when their canonical forms are equal.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct CanonicalActionGraph {
    /// Every node by its key (see [`get_node_key`]).
    pub nodes: BTreeMap<String, CanonicalNode>,
    /// Every edge as (from, to, weight), by node key. Parallel edges with the
    /// same weight are merged, as the pipeline merges every edge between the
    /// same pair of nodes.
    pub edges: BTreeSet<(String, String, &'static str)>,
    /// Keys of the nodes that run after another node, as a cleanup.
    pub cleanup_nodes: BTreeSet<String>,
    /// Keys that more than one node has, which means a node identity was
    /// inserted more than once. Always empty for a valid graph.
    pub duplicate_nodes: BTreeSet<String>,
    pub context: CanonicalActionContext,
    pub partition: Option<CanonicalPartition>,
}

impl CanonicalActionGraph {
    pub fn new(context: &ActionContext, graph: &ActionGraph) -> Self {
        let mut canonical = Self {
            context: context.into(),
            ..Default::default()
        };

        for node in graph.get_inner_nodes().values() {
            let key = get_node_key(node);

            if canonical.nodes.contains_key(&key) {
                canonical.duplicate_nodes.insert(key.clone());
            }

            canonical.nodes.insert(
                key,
                CanonicalNode {
                    label: node.label(),
                    id: node.get_id(),
                    priority: node.get_priority(),
                },
            );
        }

        let inner = graph.get_inner_graph().graph();

        for edge in inner.edge_references() {
            canonical.edges.insert((
                get_node_key_at(graph, inner[edge.source()]),
                get_node_key_at(graph, inner[edge.target()]),
                get_edge_weight(edge.weight()),
            ));
        }

        canonical.cleanup_nodes = graph
            .get_cleanup_indices()
            .iter()
            .map(|index| get_node_key_at(graph, *index))
            .collect();

        canonical
    }

    pub fn with_partition(mut self, graph: &ActionGraph, partition: &RunPartition) -> Self {
        self.partition = Some(CanonicalPartition {
            targets: partition
                .targets
                .iter()
                .map(|(index, target)| (get_node_key_at(graph, *index), target.to_string()))
                .collect(),
            size: partition.size,
        });
        self
    }

    /// Render every node, edge, and context field as one line each, in
    /// sorted order, so that differences can be printed line by line.
    pub fn to_lines(&self) -> Vec<String> {
        let mut lines = vec![];

        for (key, node) in &self.nodes {
            lines.push(format!(
                "node {key} (label={}, priority={}, id={})",
                node.label, node.priority, node.id
            ));
        }

        for key in &self.duplicate_nodes {
            lines.push(format!("duplicate node {key}"));
        }

        for (from, to, weight) in &self.edges {
            lines.push(format!("edge {from} -> {to} ({weight})"));
        }

        for key in &self.cleanup_nodes {
            lines.push(format!("cleanup node {key}"));
        }

        let context = &self.context;

        if let Some(affected) = &context.affected {
            lines.push(format!(
                "context.affected.should_check {}",
                affected.should_check
            ));

            for (id, state) in &affected.projects {
                lines.push(format!("context.affected.projects {id} {state:?}"));
            }

            for (target, state) in &affected.tasks {
                lines.push(format!("context.affected.tasks {target} {state:?}"));
            }
        }

        let mut push_set = |name: &str, set: &BTreeSet<String>| {
            for item in set {
                lines.push(format!("context.{name} {item}"));
            }
        };

        push_set("changed_files", &context.changed_files);
        push_set("cleanup_targets", &context.cleanup_targets);
        push_set("initial_targets", &context.initial_targets);
        push_set("primary_targets", &context.primary_targets);
        push_set("wait_targets", &context.wait_targets);

        for (target, deps) in &context.ignored_dependencies {
            for dep in deps {
                lines.push(format!("context.ignored_dependencies {target} -> {dep}"));
            }
        }

        for (target, state) in &context.target_states {
            lines.push(format!("context.target_states {target} {state}"));
        }

        if let Some(style) = &context.output_style {
            lines.push(format!("context.output_style {style}"));
        }

        if !context.passthrough_args.is_empty() {
            lines.push(format!(
                "context.passthrough_args {:?}",
                context.passthrough_args
            ));
        }

        if let Some(partition) = &self.partition {
            lines.push(format!("partition.size {:?}", partition.size));

            for (key, target) in &partition.targets {
                lines.push(format!("partition.targets {key} = {target}"));
            }
        }

        lines
    }

    /// The lines that are only in this form (`-`), or only in the other (`+`).
    pub fn diff(&self, other: &Self) -> Vec<String> {
        let left = self.to_lines().into_iter().collect::<BTreeSet<_>>();
        let right = other.to_lines().into_iter().collect::<BTreeSet<_>>();

        left.difference(&right)
            .map(|line| format!("- {line}"))
            .chain(right.difference(&left).map(|line| format!("+ {line}")))
            .collect()
    }
}

fn get_node_key_at(graph: &ActionGraph, index: NodeIndex) -> String {
    graph
        .get_node_from_index(&index)
        .map(get_node_key)
        .unwrap_or_else(|| format!("(missing node {})", index.index()))
}

/// Assert that 2 builds are equivalent, printing the lines that differ.
pub fn assert_canonical_eq(left: &CanonicalActionGraph, right: &CanonicalActionGraph) {
    let diff = left.diff(right);

    assert!(
        diff.is_empty(),
        "Action graphs are not equivalent (- left, + right):\n{}",
        diff.join("\n")
    );
}

/// Check the invariants that every built graph must uphold (see "Graph invariants"
/// in docs/ACTION_GRAPH.md), and return a description of every violation:
///
/// - The graph is acyclic, and each node identity has one node.
/// - When `SyncWorkspace` is enabled, every node except `SyncWorkspace` and
///   `SetupProto` reaches it.
/// - `Optional` is never an edge weight, and `Cleanup` and `Wait` edges only
///   connect `RunTask` nodes.
/// - No setup or sync node links to a `RunTask` node.
/// - Every cleanup index is a `RunTask` node with an outgoing `Cleanup` edge.
pub fn find_invariant_violations(graph: &ActionGraph, sync_workspace: bool) -> Vec<String> {
    let mut violations = vec![];
    let inner = graph.get_inner_graph().graph();
    let node_at = |index: NodeIndex| {
        graph
            .get_node_from_index(&inner[index])
            .expect("Graph index has no node!")
    };
    let is_task = |index: NodeIndex| matches!(node_at(index), ActionNode::RunTask(_));

    if petgraph::algo::toposort(inner, None).is_err() {
        violations.push("graph has a cycle".into());
    }

    if let Err(error) = graph.sort_topological() {
        violations.push(format!("graph can't be sorted: {error}"));
    }

    for key in CanonicalActionGraph::new(&ActionContext::default(), graph).duplicate_nodes {
        violations.push(format!("{key} has multiple nodes"));
    }

    for edge in inner.edge_references() {
        let (from, to) = (edge.source(), edge.target());
        let describe = || {
            format!(
                "{} -> {} ({})",
                node_at(from).label(),
                node_at(to).label(),
                get_edge_weight(edge.weight())
            )
        };

        match edge.weight() {
            TaskDependencyType::Optional => {
                violations.push(format!("optional edge {}", describe()));
            }
            TaskDependencyType::Cleanup | TaskDependencyType::Wait
                if !is_task(from) || !is_task(to) =>
            {
                violations.push(format!("non-task cleanup or wait edge {}", describe()));
            }
            _ => {}
        };

        if is_task(to) && !is_task(from) {
            violations.push(format!("non-task node links to a task {}", describe()));
        }
    }

    if sync_workspace {
        let sync_index = inner
            .node_indices()
            .find(|index| matches!(node_at(*index), ActionNode::SyncWorkspace));
        let mut reaches = FxHashSet::default();

        if let Some(sync_index) = sync_index {
            let mut queue = VecDeque::from([sync_index]);

            while let Some(index) = queue.pop_front() {
                if reaches.insert(index) {
                    queue.extend(inner.neighbors_directed(index, Direction::Incoming));
                }
            }
        }

        for index in inner.node_indices() {
            if !reaches.contains(&index)
                && !matches!(
                    node_at(index),
                    ActionNode::SyncWorkspace | ActionNode::SetupProto(_)
                )
            {
                violations.push(format!(
                    "{} doesn't reach SyncWorkspace",
                    node_at(index).label()
                ));
            }
        }
    }

    for index in graph.get_cleanup_indices() {
        let Some(graph_index) = inner
            .node_indices()
            .find(|graph_index| inner[*graph_index] == *index)
        else {
            violations.push(format!("cleanup index {} has no node", index.index()));
            continue;
        };

        if !is_task(graph_index)
            || !inner
                .edges_directed(graph_index, Direction::Outgoing)
                .any(|edge| *edge.weight() == TaskDependencyType::Cleanup)
        {
            violations.push(format!(
                "cleanup node {} has no outgoing cleanup edge",
                node_at(graph_index).label()
            ));
        }
    }

    violations
}
