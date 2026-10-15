use moon_config::TaskDependencyType;
use moon_graph_utils::*;
use moon_project::ProjectError;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_task::Task;
use moon_task_expander::{TaskExpander, TaskLookup};
use once_cell::sync::OnceCell;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use scc::hash_map::Entry;
use std::sync::{Arc, OnceLock};
use tracing::{debug, instrument};

#[derive(Clone, Debug, Default)]
pub struct TaskNode {
    pub index: NodeIndex,
    pub task: Task,
}

#[derive(Debug, Default)]
pub struct TaskGraph {
    pub context: GraphExpanderContext,

    /// Directed-acyclic graph (DAG) of non-expanded tasks and their relationships.
    /// Cycles are rejected when the graph is built, so it's not re-checked here.
    pub graph: DiGraph<NodeIndex, TaskDependencyType>,

    /// Task targets by node index. Node indexes are dense (0 to the node
    /// count), so the target of a node is at the position of its index.
    pub indexes: Vec<Target>,

    /// Map of task nodes by target.
    pub nodes: FxHashMap<Target, TaskNode>,

    /// Every target sorted, derived once, as `get_all` is called
    /// repeatedly (like by affected tracking) and must be deterministic.
    sorted_targets: OnceLock<Vec<Target>>,

    /// Project graph, required for expansion.
    project_graph: Arc<ProjectGraph>,

    /// Map of expanded tasks by target.
    tasks: Arc<scc::HashMap<Target, Arc<OnceCell<Arc<Task>>>>>,
}

impl TaskGraph {
    pub fn new(context: GraphExpanderContext, project_graph: Arc<ProjectGraph>) -> Self {
        debug!("Creating task graph");

        Self {
            context,
            project_graph,
            ..Default::default()
        }
    }

    /// Return a task with the provided target from the graph.
    /// If the task does not exist or has been misconfigured, return an error.
    #[instrument(name = "get_task", skip(self))]
    pub fn get(&self, target: &Target) -> miette::Result<Arc<Task>> {
        self.internal_get(target)
    }

    /// Return an unexpanded task with the provided target from the graph.
    pub fn get_unexpanded(&self, target: &Target) -> miette::Result<&Task> {
        let node = self
            .nodes
            .get(target)
            .ok_or_else(|| ProjectError::UnknownTask {
                task_id: target.get_task_id().unwrap().to_string(),
                project_id: target.get_project_id().unwrap().to_string(),
            })?;

        Ok(&node.task)
    }

    /// Return all tasks from the graph, sorted by target. Tasks are
    /// expanded in parallel when there are enough of them.
    #[instrument(name = "get_all_tasks", skip(self))]
    pub fn get_all(&self) -> miette::Result<Vec<Arc<Task>>> {
        let targets = self.sorted_targets.get_or_init(|| {
            let mut targets = self.nodes.keys().cloned().collect::<Vec<_>>();
            targets.sort();
            targets
        });

        self.internal_get_many(&targets.iter().collect::<Vec<_>>())
    }

    /// Return all unexpanded tasks from the graph.
    pub fn get_all_unexpanded(&self) -> miette::Result<Vec<&Task>> {
        Ok(self.nodes.values().map(|node| &node.task).collect())
    }

    /// Return many tasks from the graph by target, in the order provided.
    /// Tasks are expanded in parallel when there are enough of them.
    #[instrument(name = "get_many_tasks", skip(self))]
    pub fn get_many(&self, targets: &[Target]) -> miette::Result<Vec<Arc<Task>>> {
        self.internal_get_many(&targets.iter().collect::<Vec<_>>())
    }

    /// Return many unexpanded tasks from the graph.
    pub fn get_many_unexpanded(&self, targets: &[Target]) -> miette::Result<Vec<&Task>> {
        let mut many = vec![];

        for target in targets {
            many.push(self.get_unexpanded(target)?);
        }

        Ok(many)
    }

    /// Focus the graph for a specific task by target, including all of its
    /// dependencies, and its dependents within the provided depth.
    pub fn focus_for(&self, target: &Target, dependents: Depth) -> miette::Result<Self> {
        let task = self.get(target)?;
        let graph = self.to_focused_graph(&task, dependents);
        let (nodes, edges) = graph.into_nodes_edges();

        let mut dag = DiGraph::with_capacity(nodes.len(), edges.len());
        let mut indexes = Vec::with_capacity(nodes.len());
        let mut tasks = FxHashMap::default();

        // The focused graph has different node inndexes,
        // so we need to update our internal structures to match
        for (i, node) in nodes.into_iter().enumerate() {
            let new_index = NodeIndex::from(i as u32);
            let old_index = node.weight;
            let target = &self.indexes[old_index.index()];

            indexes.push(target.to_owned());

            tasks.insert(
                target.to_owned(),
                TaskNode {
                    index: new_index,
                    task: self.get_node_by_index(&old_index).to_owned(),
                },
            );

            dag.add_node(new_index);
        }

        // A subset of an acyclic graph is acyclic, so the edges are inserted
        // without checking for cycles, but still deduplicated
        for edge in edges {
            dag.update_edge(edge.source(), edge.target(), edge.weight);
        }

        Ok(Self {
            indexes,
            context: self.context.clone(),
            graph: dag,
            nodes: tasks,
            project_graph: self.project_graph.clone(),
            sorted_targets: OnceLock::new(),
            tasks: self.tasks.clone(),
        })
    }

    /// Expand the provided targets and return their tasks in the same order.
    /// When there are enough targets, they are expanded across threads in a
    /// dependencies-first order, so that a thread expanding a dependent rarely
    /// waits on another thread that is still expanding its dependency. The
    /// graph is acyclic, so these waits can never deadlock. If expansion fails,
    /// the error of the first failing target (in the provided order) is returned,
    /// regardless of which thread failed first.
    fn internal_get_many(&self, targets: &[&Target]) -> miette::Result<Vec<Arc<Task>>> {
        let mut results = Vec::with_capacity(targets.len());
        results.resize_with(targets.len(), || None);

        // Already expanded tasks are returned directly, so that repeated
        // calls (like by affected tracking) don't spawn threads for cache hits
        let mut pending = vec![];

        for (position, target) in targets.iter().enumerate() {
            match self.get_expanded(target) {
                Some(task) => results[position] = Some(Ok(task)),
                None => pending.push(position),
            }
        }

        if should_run_in_parallel(pending.len()) {
            let ranks = self.dependency_first_ranks();

            pending.sort_by_cached_key(|position| {
                self.nodes
                    .get(targets[*position])
                    .and_then(|node| ranks.get(node.index.index()).copied())
                    .unwrap_or(usize::MAX)
            });

            for (position, result) in pending.iter().zip(map_in_parallel(&pending, |position| {
                self.internal_get(targets[*position])
            })) {
                results[*position] = Some(result);
            }
        } else {
            for position in pending {
                results[position] = Some(self.internal_get(targets[position]));
            }
        }

        results
            .into_iter()
            .map(|result| result.expect("Every target is expanded!"))
            .collect()
    }

    /// Return the rank of each node (by graph index) in a dependencies-first
    /// topological order. Ranks are only used to order work, so if sorting
    /// fails, all nodes share the same rank.
    fn dependency_first_ranks(&self) -> Vec<usize> {
        let mut ranks = vec![0; self.graph.node_count()];

        if let Ok(indices) = toposort(&self.graph, None) {
            // Edges point from a task to its dependency,
            // so reverse the order to get dependencies first
            for (rank, index) in indices.into_iter().rev().enumerate() {
                ranks[index.index()] = rank;
            }
        }

        ranks
    }

    /// Return the task if it has already been expanded. This is the hot path
    /// for graph traversals, so it reads without locking the bucket or cloning
    /// the target.
    fn get_expanded(&self, target: &Target) -> Option<Arc<Task>> {
        self.tasks
            .read_sync(target, |_, once| once.get().cloned())
            .flatten()
    }

    fn internal_get(&self, target: &Target) -> miette::Result<Arc<Task>> {
        if let Some(task) = self.get_expanded(target) {
            return Ok(task);
        }

        let once = match self.tasks.entry_sync(target.to_owned()) {
            Entry::Occupied(e) => Arc::clone(e.get()),
            Entry::Vacant(e) => {
                let once = Arc::new(OnceCell::new());
                e.insert_entry(Arc::clone(&once));
                once
            }
        };

        once.get_or_try_init(|| {
            let expander = TaskExpander::new(
                &self.project_graph,
                self.project_graph
                    .get_unexpanded(target.get_project_id()?)?,
                &self.context,
                self,
            );
            Ok(Arc::new(expander.expand(self.get_unexpanded(target)?)?))
        })
        .map(Arc::clone)
    }
}

impl TaskLookup for TaskGraph {
    fn get_task(&self, target: &Target) -> miette::Result<Arc<Task>> {
        self.internal_get(target)
    }
}

impl GraphData<Task, TaskDependencyType, Target> for TaskGraph {
    fn get_graph(&self) -> &DiGraph<NodeIndex, TaskDependencyType> {
        &self.graph
    }

    fn get_nodes(&self) -> FxHashMap<NodeIndex, &Task> {
        self.nodes
            .values()
            .map(|node| (node.index, &node.task))
            .collect()
    }

    fn get_node_by_index(&self, index: &NodeIndex) -> &Task {
        &self.nodes[&self.indexes[index.index()]].task
    }

    fn get_node_key(&self, node: &Task) -> Target {
        node.target.clone()
    }
}

impl GraphConnections<Task, TaskDependencyType, Target> for TaskGraph {
    fn get_node_index(&self, node: &Task) -> NodeIndex {
        self.nodes[&node.target].index
    }
}

impl GraphConversions<Task, TaskDependencyType, Target> for TaskGraph {}

impl GraphToDot<Task, TaskDependencyType, Target> for TaskGraph {}

impl GraphToJson<Task, TaskDependencyType, Target> for TaskGraph {}
