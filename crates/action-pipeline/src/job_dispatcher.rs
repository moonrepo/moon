use crate::job_context::JobContext;
use moon_action::ActionNode;
use moon_action_graph::ActionGraph;
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::debug;

pub struct JobDispatcher<'graph> {
    context: JobContext,
    completed_queue: UnboundedReceiver<NodeIndex>,
    nodes: &'graph FxHashMap<NodeIndex, ActionNode>,
    groups: BTreeMap<u8, Vec<NodeIndex>>, // topo

    /// The dependencies and dependents of every node, extracted from the graph
    /// once up front (indexed by node index), so that dispatching works from
    /// these lists alone, instead of traversing the graph on every check.
    dependencies: Vec<Vec<NodeIndex>>,
    dependents: Vec<Vec<NodeIndex>>,

    /// How many dependencies of each node have yet to complete. Decremented as
    /// jobs complete, so that "can this node be dispatched" is a lookup,
    /// instead of a walk of all its dependencies.
    blocked_by: Vec<usize>,

    /// How many nodes are currently unblocked but have yet to be dispatched.
    /// When none are, there's nothing to scan for, as a node can only be
    /// dispatched once all of its dependencies have completed.
    ready_nodes: usize,

    completed: FxHashSet<NodeIndex>,
    visited: FxHashSet<NodeIndex>,
    total_nodes: usize,
}

impl<'graph> JobDispatcher<'graph> {
    pub fn new(
        action_graph: &'graph ActionGraph,
        context: JobContext,
        groups: BTreeMap<u8, Vec<NodeIndex>>,
        completed_queue: UnboundedReceiver<NodeIndex>,
    ) -> Self {
        let graph = action_graph.get_inner_graph().graph();
        let total_nodes = graph.node_count();

        let mut dependencies = vec![Vec::new(); total_nodes];
        let mut dependents = vec![Vec::new(); total_nodes];
        let mut blocked_by = vec![0; total_nodes];

        for index in graph.node_indices() {
            let mut node_deps: Vec<NodeIndex> = vec![];

            // Walked in the graph's own order, so that dispatching
            // remains deterministic. Duplicate and self edges are
            // dropped, as they would never be unblocked
            for dep_index in graph.neighbors_directed(index, Direction::Outgoing) {
                if dep_index != index && !node_deps.contains(&dep_index) {
                    node_deps.push(dep_index);
                }
            }

            for dep_index in &node_deps {
                dependents[dep_index.index()].push(index);
            }

            blocked_by[index.index()] = node_deps.len();
            dependencies[index.index()] = node_deps;
        }

        Self {
            context,
            completed_queue,
            nodes: action_graph.get_inner_nodes(),
            groups,
            dependencies,
            dependents,
            ready_nodes: blocked_by.iter().filter(|count| **count == 0).count(),
            blocked_by,
            completed: FxHashSet::default(),
            visited: FxHashSet::default(),
            total_nodes,
        }
    }

    pub fn has_queued_jobs(&self) -> bool {
        self.visited.len() < self.total_nodes
    }

    /// Drain the jobs that have completed since the last dispatch, and unblock
    /// their dependents. A job may be marked as completed more than once (a
    /// persistent job is marked when dispatched, and again when it exits), so
    /// only the first completion is counted.
    fn drain_completed_jobs(&mut self) {
        while let Ok(index) = self.completed_queue.try_recv() {
            if !self.completed.insert(index) {
                continue;
            }

            let Some(dependents) = self.dependents.get(index.index()) else {
                continue;
            };

            for dependent_index in dependents {
                if let Some(count) = self.blocked_by.get_mut(dependent_index.index()) {
                    *count = count.saturating_sub(1);

                    if *count == 0 {
                        self.ready_nodes += 1;
                    }
                }
            }
        }
    }

    fn is_dispatchable(&self, index: NodeIndex) -> bool {
        self.blocked_by
            .get(index.index())
            .is_none_or(|count| *count == 0)
    }

    fn find_applicable_index(
        &self,
        group: u8,
        index: NodeIndex,
        traversed: &mut FxHashSet<NodeIndex>,
    ) -> Option<NodeIndex> {
        if !traversed.insert(index)
            || self.visited.contains(&index)
            || self.completed.contains(&index)
        {
            return None;
        }

        // Ensure all dependencies of the index have
        // completed before dispatching
        if self.is_dispatchable(index) {
            return Some(index);
        }

        // If not all dependencies have completed yet,
        // attempt to find a dependency to run
        if group < 2
            && let Some(dependencies) = self.dependencies.get(index.index())
        {
            for dep_index in dependencies {
                if let Some(index) = self.find_applicable_index(group, *dep_index, traversed) {
                    return Some(index);
                }
            }
        }

        // Otherwise do nothing
        None
    }
}

// This is based on the `Topo` struct from petgraph!
impl JobDispatcher<'_> {
    pub async fn next(&mut self) -> Option<NodeIndex> {
        self.drain_completed_jobs();

        // Everything that remains is waiting on a job that is still running,
        // so avoid scanning the groups entirely
        if self.ready_nodes == 0 {
            self.remove_dispatched_jobs();

            return None;
        }

        // Avoid repeatedly traversing the same blocked dependency subgraph
        // while a prerequisite action is still running.
        let mut traversed = FxHashSet::default();

        // Loop based on priority groups, from critical to low
        {
            for (group, indices) in &self.groups {
                // Then loop through the indices within the group,
                // which are topologically sorted
                for maybe_index in indices {
                    let Some(index) =
                        self.find_applicable_index(*group, *maybe_index, &mut traversed)
                    else {
                        continue;
                    };

                    if let Some(node) = self.nodes.get(&index) {
                        let id = node.get_id();

                        // If the same exact action is currently running,
                        // avoid running another in parallel to avoid weird
                        // collisions. This is especially true for `RunTask`,
                        // where different args/env vars run the same task,
                        // but with slightly different variance.
                        if id > 0 && node.is_standard() {
                            if let Some(running_index) = self
                                .context
                                .running_jobs
                                .read()
                                .await
                                .iter()
                                .find(|(_, running_id)| *running_id == &id)
                            {
                                debug!(
                                    index = index.index(),
                                    running_index = running_index.0.index(),
                                    "Another job of a similar type is currently running, deferring dispatch",
                                );

                                continue;
                            }

                            self.context.running_jobs.write().await.insert(index, id);
                        }
                    }

                    debug!(index = index.index(), "Dispatching job");

                    self.visited.insert(index);
                    self.ready_nodes = self.ready_nodes.saturating_sub(1);

                    return Some(index);
                }
            }
        }

        self.remove_dispatched_jobs();

        None
    }

    // Remove indices and groups once they have been dispatched,
    // so that the next pass has less to scan through
    fn remove_dispatched_jobs(&mut self) {
        let visited = &self.visited;

        self.groups.retain(|_, indices| {
            indices.retain(|index| !visited.contains(index));

            !indices.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_emitter::EventEmitter;
    use moon_action::{Action, ActionNode, RunTaskNode, SyncProjectNode};
    use moon_action_graph::{ActionGraph, ActionGraphType};
    use moon_common::Id;
    use moon_config::TaskDependencyType;
    use moon_task::Target;
    use moon_workspace_graph::WorkspaceGraph;
    use rustc_hash::FxHashMap;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::{RwLock, Semaphore, mpsc};
    use tokio_util::sync::CancellationToken;

    async fn create_job_context() -> (JobContext, UnboundedReceiver<NodeIndex>) {
        let (sender, _receiver) = mpsc::channel::<Action>(8);
        let (completed_sender, completed_receiver) = mpsc::unbounded_channel::<NodeIndex>();

        let context = JobContext {
            abort_token: CancellationToken::new(),
            bail: false,
            cancel_token: CancellationToken::new(),
            completed_queue: completed_sender,
            daemon_client: None,
            emitter: Arc::new(EventEmitter::default()),
            result_sender: sender,
            running_jobs: Arc::new(RwLock::new(FxHashMap::default())),
            semaphore: Arc::new(Semaphore::new(1)),
            workspace_graph: Arc::new(WorkspaceGraph::default()),
        };

        (context, completed_receiver)
    }

    fn create_dense_sync_graph(depth: usize, width: usize) -> ActionGraph {
        let mut graph = ActionGraphType::new();
        let mut nodes = FxHashMap::default();

        let root = graph.add_node(NodeIndex::new(0));
        nodes.insert(root, ActionNode::sync_workspace());

        let mut layers = vec![];

        for layer in 0..depth {
            let mut indices = vec![];

            for node in 0..width {
                let index = graph.add_node(NodeIndex::new(graph.node_count()));
                nodes.insert(
                    index,
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw(format!("p{layer}-{node}")),
                    }),
                );
                indices.push(index);
            }

            layers.push(indices);
        }

        for (layer_index, layer) in layers.iter().enumerate() {
            if layer_index == depth - 1 {
                for index in layer {
                    graph
                        .add_edge(*index, root, TaskDependencyType::Required)
                        .unwrap();
                }
            }

            if let Some(next_layer) = layers.get(layer_index + 1) {
                for index in layer {
                    for next_index in next_layer {
                        graph
                            .add_edge(*index, *next_index, TaskDependencyType::Required)
                            .unwrap();
                    }
                }
            }
        }

        let run_task = graph.add_node(NodeIndex::new(graph.node_count()));
        nodes.insert(
            run_task,
            ActionNode::run_task(RunTaskNode::new(Target::parse("root:noop").unwrap())),
        );

        for index in &layers[0] {
            graph
                .add_edge(run_task, *index, TaskDependencyType::Required)
                .unwrap();
        }

        ActionGraph::new(graph, nodes)
    }

    // `a` and `b` are both required by `t`
    fn create_shared_dependency_graph() -> ActionGraph {
        let mut graph = ActionGraphType::new();
        let mut nodes = FxHashMap::default();

        let mut add = |graph: &mut ActionGraphType, id: &str| {
            let index = graph.add_node(NodeIndex::new(graph.node_count()));

            nodes.insert(
                index,
                ActionNode::run_task(RunTaskNode::new(
                    Target::parse(&format!("root:{id}")).unwrap(),
                )),
            );

            index
        };

        let a = add(&mut graph, "a");
        let b = add(&mut graph, "b");
        let t = add(&mut graph, "t");

        graph.add_edge(t, a, TaskDependencyType::Required).unwrap();
        graph.add_edge(t, b, TaskDependencyType::Required).unwrap();

        ActionGraph::new(graph, nodes)
    }

    // Jobs can be marked as completed more than once — a persistent job is
    // marked when it is dispatched, and again when it exits — which must not
    // unblock a dependent that is still waiting on its other dependencies
    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_unblock_from_duplicate_completions() {
        let action_graph = create_shared_dependency_graph();
        let groups = action_graph.group_priorities(action_graph.sort_topological().unwrap());
        let (context, completed) = create_job_context().await;
        let mut dispatcher = JobDispatcher::new(&action_graph, context.clone(), groups, completed);

        let first = dispatcher.next().await.unwrap();
        let second = dispatcher.next().await.unwrap();

        assert_ne!(first, second);
        assert!(dispatcher.next().await.is_none());

        context.mark_completed(first).await;
        context.mark_completed(first).await;

        assert!(
            dispatcher.next().await.is_none(),
            "dispatched before all dependencies completed"
        );

        context.mark_completed(second).await;

        assert_eq!(dispatcher.next().await, Some(NodeIndex::new(2)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dispatches_dense_sync_graph() {
        for depth in [8, 12, 16, 20, 24] {
            let action_graph = create_dense_sync_graph(depth, 2);
            let groups = action_graph.group_priorities(action_graph.sort_topological().unwrap());
            let (context, completed) = create_job_context().await;
            let mut dispatcher =
                JobDispatcher::new(&action_graph, context.clone(), groups, completed);
            let mut dispatched = vec![];

            while dispatcher.has_queued_jobs() {
                let Some(index) = dispatcher.next().await else {
                    break;
                };

                dispatched.push(index.index());
                context.mark_completed(index).await;
            }

            assert_eq!(dispatched.len(), action_graph.get_node_count());
        }
    }

    // Mimics the pipeline loop, where jobs are dispatched until the graph is
    // blocked, and only then is a running job allowed to finish. Every node
    // must still be dispatched, and only once
    #[tokio::test(flavor = "multi_thread")]
    async fn dispatches_while_jobs_are_running() {
        let action_graph = create_dense_sync_graph(12, 3);
        let groups = action_graph.group_priorities(action_graph.sort_topological().unwrap());
        let (context, completed) = create_job_context().await;
        let mut dispatcher = JobDispatcher::new(&action_graph, context.clone(), groups, completed);
        let mut dispatched = vec![];
        let mut running: Vec<NodeIndex> = vec![];

        while dispatcher.has_queued_jobs() {
            match dispatcher.next().await {
                Some(index) => {
                    dispatched.push(index);
                    running.push(index);
                }
                // Blocked, so let a single running job finish
                None => match running.pop() {
                    Some(index) => context.mark_completed(index).await,
                    None => break,
                },
            }
        }

        assert_eq!(dispatched.len(), action_graph.get_node_count());
        assert_eq!(
            dispatched.iter().collect::<FxHashSet<_>>().len(),
            action_graph.get_node_count()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn avoids_rewalking_blocked_sync_subgraphs() {
        let action_graph = create_dense_sync_graph(12, 4);
        let groups = action_graph.group_priorities(action_graph.sort_topological().unwrap());
        let (context, completed) = create_job_context().await;
        let mut dispatcher = JobDispatcher::new(&action_graph, context, groups, completed);

        assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

        let start = Instant::now();
        let next = dispatcher.next().await;
        let elapsed = start.elapsed();

        assert!(next.is_none());
        assert!(
            elapsed < Duration::from_millis(500),
            "dispatcher search took {:?} on a blocked sync-heavy graph",
            elapsed
        );
    }
}
