use crate::job_context::{CompletedJob, JobContext};
use moon_action::ActionNode;
use moon_action_graph::ActionGraph;
use moon_config::TaskDependencyType;
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::debug;

/// How a node relates to one of its dependencies, derived from the
/// weights of the edges between them (there may be more than one).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Relation {
    /// The dependency must complete before the node can run.
    required: bool,

    /// The node cleans up after the dependency, so must wait for it
    /// to complete (a `cleanup` task dependency, which is reversed).
    cleanup: bool,

    /// The dependency must only start before the node can run.
    wait: bool,
}

impl Relation {
    fn add(&mut self, edge: &TaskDependencyType) {
        match edge {
            TaskDependencyType::Cleanup => self.cleanup = true,
            TaskDependencyType::Wait => self.wait = true,
            TaskDependencyType::Required | TaskDependencyType::Optional => self.required = true,
        };
    }

    /// Whether the node is unblocked once the dependency has been dispatched
    /// (started), instead of once it has completed, which requires every
    /// relationship between them to only wait on it.
    fn is_released_on_dispatch(&self) -> bool {
        self.wait && !self.required && !self.cleanup
    }
}

pub struct JobDispatcher<'graph> {
    context: JobContext,
    completed_queue: UnboundedReceiver<CompletedJob>,
    nodes: &'graph FxHashMap<NodeIndex, ActionNode>,
    groups: BTreeMap<u8, Vec<NodeIndex>>, // topo

    /// Nodes that clean up after the nodes they depend on, and can
    /// still run after the pipeline was aborted.
    cleanups: &'graph FxHashSet<NodeIndex>,

    /// The dependencies and dependents of every node, extracted from the graph
    /// once up front (indexed by node index), so that dispatching works from
    /// these lists alone, instead of traversing the graph on every check.
    dependencies: Vec<Vec<(NodeIndex, Relation)>>,
    dependents: Vec<Vec<NodeIndex>>,

    /// Dependents that are unblocked once the node has been dispatched,
    /// instead of once it has completed, as they only wait on it to start.
    wait_dependents: Vec<Vec<NodeIndex>>,

    /// How many dependencies of each node have yet to complete (or start, when
    /// only waited on). Decremented as jobs complete (or are dispatched), so
    /// that "can this node be dispatched" is a lookup, instead of a walk of
    /// all its dependencies.
    blocked_by: Vec<usize>,

    /// How many nodes are currently unblocked but have yet to be dispatched.
    /// When none are, there's nothing to scan for, as a node can only be
    /// dispatched once all of its dependencies have completed (or started).
    ready_nodes: usize,

    completed: FxHashSet<NodeIndex>,

    /// Completed nodes whose action executed a task's command.
    executed: FxHashSet<NodeIndex>,

    /// Completed nodes whose action failed, or was aborted.
    failed: FxHashSet<NodeIndex>,

    /// Nodes that have released their wait dependents.
    released: FxHashSet<NodeIndex>,

    /// Completed nodes whose action started running, instead of being
    /// aborted or skipped before it could start.
    started: FxHashSet<NodeIndex>,

    visited: FxHashSet<NodeIndex>,
    total_nodes: usize,
}

impl<'graph> JobDispatcher<'graph> {
    pub fn new(
        action_graph: &'graph ActionGraph,
        context: JobContext,
        groups: BTreeMap<u8, Vec<NodeIndex>>,
        completed_queue: UnboundedReceiver<CompletedJob>,
    ) -> Self {
        let graph = action_graph.get_inner_graph().graph();
        let total_nodes = graph.node_count();

        let mut dependencies = vec![Vec::new(); total_nodes];
        let mut dependents = vec![Vec::new(); total_nodes];
        let mut wait_dependents = vec![Vec::new(); total_nodes];
        let mut blocked_by = vec![0; total_nodes];

        for index in graph.node_indices() {
            let mut node_deps: Vec<(NodeIndex, Relation)> = vec![];

            // Walked in the graph's own order, so that dispatching remains
            // deterministic. Duplicate edges are merged into a single relation,
            // and self edges are dropped, as they would never be unblocked
            for edge in graph.edges_directed(index, Direction::Outgoing) {
                let dep_index = edge.target();

                if dep_index == index {
                    continue;
                }

                match node_deps.iter_mut().find(|(i, _)| *i == dep_index) {
                    Some((_, relation)) => relation.add(edge.weight()),
                    None => {
                        let mut relation = Relation::default();
                        relation.add(edge.weight());

                        node_deps.push((dep_index, relation));
                    }
                };
            }

            for (dep_index, relation) in &node_deps {
                if relation.is_released_on_dispatch() {
                    wait_dependents[dep_index.index()].push(index);
                } else {
                    dependents[dep_index.index()].push(index);
                }
            }

            blocked_by[index.index()] = node_deps.len();
            dependencies[index.index()] = node_deps;
        }

        Self {
            context,
            completed_queue,
            nodes: action_graph.get_inner_nodes(),
            groups,
            cleanups: action_graph.get_cleanup_indices(),
            dependencies,
            dependents,
            wait_dependents,
            ready_nodes: blocked_by.iter().filter(|count| **count == 0).count(),
            blocked_by,
            completed: FxHashSet::default(),
            executed: FxHashSet::default(),
            failed: FxHashSet::default(),
            released: FxHashSet::default(),
            started: FxHashSet::default(),
            visited: FxHashSet::default(),
            total_nodes,
        }
    }

    pub fn has_queued_jobs(&self) -> bool {
        self.visited.len() < self.total_nodes
    }

    /// Whether other nodes only wait on the node to start, instead of complete.
    pub fn has_wait_dependents(&self, index: NodeIndex) -> bool {
        self.wait_dependents
            .get(index.index())
            .is_some_and(|dependents| !dependents.is_empty())
    }

    /// Drain the jobs that have completed since the last dispatch, and unblock
    /// their dependents. A job may be marked as completed more than once (a
    /// persistent job is marked when dispatched, and again when it exits), so
    /// only the first completion is counted.
    fn drain_completed_jobs(&mut self) {
        while let Ok(job) = self.completed_queue.try_recv() {
            if !self.completed.insert(job.index) {
                continue;
            }

            if job.started {
                self.started.insert(job.index);
            }

            if job.executed {
                self.executed.insert(job.index);
            }

            if job.failed {
                self.failed.insert(job.index);
            }

            // Every job is dispatched before it completes, which releases the
            // dependents that only wait on it, but ensure they are regardless
            self.release_wait_dependents(job.index);

            if let Some(dependents) = self.dependents.get(job.index.index()) {
                unblock(&mut self.blocked_by, &mut self.ready_nodes, dependents);
            }
        }
    }

    /// Unblock the dependents that only wait on the node to start. This happens
    /// once it has been dispatched (or has completed), whichever is first.
    fn release_wait_dependents(&mut self, index: NodeIndex) {
        if !self.released.insert(index) {
            return;
        }

        if let Some(dependents) = self.wait_dependents.get(index.index()) {
            unblock(&mut self.blocked_by, &mut self.ready_nodes, dependents);
        }
    }

    fn is_dispatchable(&self, index: NodeIndex) -> bool {
        self.blocked_by
            .get(index.index())
            .is_none_or(|count| *count == 0)
    }

    fn is_running(&self, index: &NodeIndex) -> bool {
        self.visited.contains(index) && !self.completed.contains(index)
    }

    fn is_task(&self, index: &NodeIndex) -> bool {
        matches!(self.nodes.get(index), Some(ActionNode::RunTask(_)))
    }

    /// The ID of an action that must not run in parallel with itself.
    fn get_exclusive_id(&self, index: &NodeIndex) -> Option<u64> {
        self.nodes.get(index).and_then(|node| {
            let id = node.get_id();

            (id > 0 && node.is_standard()).then_some(id)
        })
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
            for (dep_index, _) in dependencies {
                if let Some(index) = self.find_applicable_index(group, *dep_index, traversed) {
                    return Some(index);
                }
            }
        }

        // Otherwise do nothing
        None
    }

    /// Take the cleanup jobs that can still run after the pipeline was aborted,
    /// and mark them as dispatched. Jobs that were not dispatched before the
    /// abort will never run now, so a cleanup job can run once:
    ///
    /// - Its own dependencies have started and completed, and those that are
    ///   not tasks (like installing dependencies) have passed.
    /// - The jobs it cleans up after are no longer running (or pending when they
    ///   are cleanup jobs themselves), and at least one of them started (there's
    ///   nothing to clean up otherwise).
    ///
    /// When giving up, jobs that are still running are no longer waited on, and
    /// are treated as if they will never complete (unless they're cleanup jobs).
    ///
    /// This should be called repeatedly (as jobs complete) until nothing is
    /// returned, and [`Self::is_waiting_on_cleanups`] is false.
    pub fn take_cleanups_after_abort(&mut self, give_up: bool) -> Vec<NodeIndex> {
        self.drain_completed_jobs();

        // Sorted for deterministic dispatching, and binary searching
        let mut pending = self
            .cleanups
            .iter()
            .filter(|index| !self.visited.contains(index))
            .copied()
            .collect::<Vec<_>>();

        pending.sort();

        // Like when dispatching normally, the same action can't run in parallel
        let mut running_ids = self
            .cleanups
            .iter()
            .filter(|index| self.is_running(index))
            .filter_map(|index| self.get_exclusive_id(index))
            .collect::<FxHashSet<_>>();

        let mut deferred = FxHashSet::default();
        let mut taken = vec![];

        while !pending.is_empty() {
            let runnable = pending
                .iter()
                .filter(|index| self.can_cleanup_after_abort(**index, &pending, give_up))
                .copied()
                .collect::<Vec<_>>();

            let mut progressed = false;

            for index in runnable {
                if let Some(id) = self.get_exclusive_id(&index)
                    && !running_ids.insert(id)
                {
                    deferred.insert(index);
                    continue;
                }

                if self.is_dispatchable(index) {
                    self.ready_nodes = self.ready_nodes.saturating_sub(1);
                }

                self.visited.insert(index);
                self.release_wait_dependents(index);

                pending.retain(|i| *i != index);
                taken.push(index);
                progressed = true;
            }

            // Taken jobs may unblock others that only wait on them to start
            if progressed {
                continue;
            }

            // Otherwise nothing else can run right now, but pending jobs may be
            // blocked by other pending jobs that can never run, as everything
            // they relate to has finished, so remove those and try again
            let dead = pending
                .iter()
                .filter(|index| {
                    !deferred.contains(index)
                        && !self.relates_to_unfinished(**index, &pending, give_up)
                })
                .copied()
                .collect::<FxHashSet<_>>();

            if dead.is_empty() {
                break;
            }

            pending.retain(|index| !dead.contains(index));
        }

        taken.sort();
        taken
    }

    /// Whether cleanup jobs are still running, or pending cleanup jobs are
    /// waiting on running jobs to complete (unless giving up on them).
    pub fn is_waiting_on_cleanups(&mut self, give_up: bool) -> bool {
        self.drain_completed_jobs();

        self.cleanups.iter().any(|index| {
            if self.visited.contains(index) {
                return !self.completed.contains(index);
            }

            self.dependencies
                .get(index.index())
                .is_some_and(|dependencies| {
                    dependencies.iter().any(|(dep_index, _)| {
                        self.is_running(dep_index)
                            && (!give_up || self.cleanups.contains(dep_index))
                    })
                })
        })
    }

    fn can_cleanup_after_abort(
        &self,
        index: NodeIndex,
        pending: &[NodeIndex],
        give_up: bool,
    ) -> bool {
        let Some(dependencies) = self.dependencies.get(index.index()) else {
            return false;
        };

        let mut cleans_up_executed = false;

        for (dep_index, relation) in dependencies {
            let completed = self.completed.contains(dep_index);
            let started = completed && self.started.contains(dep_index);

            if relation.required {
                // Its own dependencies must have ran, and other actions must
                // have passed, as only task dependencies are checked when ran,
                // otherwise it would run against a broken environment
                if !started || (!self.is_task(dep_index) && self.failed.contains(dep_index)) {
                    return false;
                }

                if relation.cleanup && self.executed.contains(dep_index) {
                    cleans_up_executed = true;
                }
            } else if relation.cleanup {
                // It ran its command, so there's something to clean up
                if completed {
                    if self.executed.contains(dep_index) {
                        cleans_up_executed = true;
                    }
                }
                // Still running, or another cleanup job that may run first
                else if self.is_unfinished(dep_index, pending, give_up) {
                    return false;
                }
                // Otherwise it never started, and never will
            }
            // Wait dependencies must have started, or still be running
            else if !started && !self.is_running(dep_index) {
                return false;
            }
        }

        cleans_up_executed
    }

    /// Whether a cleanup job has nothing to clean up, as none of the jobs it
    /// cleans up after executed their command (they were skipped or cached),
    /// and no other job depends on it. This must be called once all of
    /// its dependencies have completed (it has been dispatched).
    pub fn is_needless_cleanup(&self, index: NodeIndex) -> bool {
        if !self.cleanups.contains(&index) {
            return false;
        }

        let Some(dependencies) = self.dependencies.get(index.index()) else {
            return false;
        };

        // Other jobs may require (or wait on) it, regardless of cleaning up
        let is_depended_on = self
            .dependents
            .get(index.index())
            .into_iter()
            .chain(self.wait_dependents.get(index.index()))
            .flatten()
            .any(|dependent_index| {
                self.dependencies
                    .get(dependent_index.index())
                    .is_some_and(|dependent_deps| {
                        dependent_deps.iter().any(|(dep_index, relation)| {
                            *dep_index == index && (relation.required || relation.wait)
                        })
                    })
            });

        !is_depended_on
            && !dependencies
                .iter()
                .any(|(dep_index, relation)| relation.cleanup && self.executed.contains(dep_index))
    }

    /// Whether the node may still complete: it's still running, or it's another
    /// pending cleanup job. When giving up, only cleanup jobs may still complete.
    fn is_unfinished(&self, index: &NodeIndex, pending: &[NodeIndex], give_up: bool) -> bool {
        pending.binary_search(index).is_ok()
            || (self.is_running(index) && (!give_up || self.cleanups.contains(index)))
    }

    fn relates_to_unfinished(
        &self,
        index: NodeIndex,
        pending: &[NodeIndex],
        give_up: bool,
    ) -> bool {
        self.dependencies
            .get(index.index())
            .is_some_and(|dependencies| {
                dependencies
                    .iter()
                    .any(|(dep_index, _)| self.is_unfinished(dep_index, pending, give_up))
            })
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

        let Some(index) = self.find_next_index().await else {
            self.remove_dispatched_jobs();

            return None;
        };

        debug!(index = index.index(), "Dispatching job");

        self.visited.insert(index);
        self.ready_nodes = self.ready_nodes.saturating_sub(1);

        // Dependents that only wait on this job to start can now run
        self.release_wait_dependents(index);

        Some(index)
    }

    async fn find_next_index(&self) -> Option<NodeIndex> {
        // Avoid repeatedly traversing the same blocked dependency subgraph
        // while a prerequisite action is still running.
        let mut traversed = FxHashSet::default();

        // Loop based on priority groups, from critical to low
        for (group, indices) in &self.groups {
            // Then loop through the indices within the group,
            // which are topologically sorted
            for maybe_index in indices {
                let Some(index) = self.find_applicable_index(*group, *maybe_index, &mut traversed)
                else {
                    continue;
                };

                // Once the pipeline was aborted, cleanup jobs are ran separately,
                // as the jobs they clean up after may have never started
                if self.cleanups.contains(&index) && self.context.abort_token.is_cancelled() {
                    continue;
                }

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

                return Some(index);
            }
        }

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

fn unblock(blocked_by: &mut [usize], ready_nodes: &mut usize, dependents: &[NodeIndex]) {
    for dependent_index in dependents {
        if let Some(count) = blocked_by.get_mut(dependent_index.index())
            && *count > 0
        {
            *count -= 1;

            if *count == 0 {
                *ready_nodes += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abort_state::AbortState;
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

    async fn create_job_context() -> (JobContext, UnboundedReceiver<CompletedJob>) {
        let (sender, _receiver) = mpsc::channel::<Action>(8);
        let (completed_sender, completed_receiver) = mpsc::unbounded_channel::<CompletedJob>();

        let context = JobContext {
            abort_token: CancellationToken::new(),
            abort_state: Arc::new(AbortState::default()),
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

    fn create_dispatcher(
        action_graph: &ActionGraph,
        context: JobContext,
        completed: UnboundedReceiver<CompletedJob>,
    ) -> JobDispatcher<'_> {
        let groups = action_graph.group_priorities(action_graph.sort_topological().unwrap());

        JobDispatcher::new(action_graph, context, groups, completed)
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

    /// Create a graph of `root:*` task nodes (or a project sync node for `sync`),
    /// with the provided edges between them, where the cleanup set is derived
    /// from the cleanup edges.
    fn create_task_graph(
        ids: &[&str],
        edges: &[(usize, usize, TaskDependencyType)],
    ) -> ActionGraph {
        let mut graph = ActionGraphType::new();
        let mut nodes = FxHashMap::default();
        let mut cleanups = FxHashSet::default();

        for id in ids {
            let index = graph.add_node(NodeIndex::new(graph.node_count()));

            nodes.insert(
                index,
                if *id == "sync" {
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("root"),
                    })
                } else {
                    ActionNode::run_task(RunTaskNode::new(
                        Target::parse(&format!("root:{id}")).unwrap(),
                    ))
                },
            );
        }

        for (from, to, edge) in edges {
            let from = NodeIndex::new(*from);

            graph.add_edge(from, NodeIndex::new(*to), *edge).unwrap();

            if matches!(edge, TaskDependencyType::Cleanup) {
                cleanups.insert(from);
            }
        }

        ActionGraph::new_with_cleanups(graph, nodes, cleanups)
    }

    fn indexes(list: &[usize]) -> Vec<NodeIndex> {
        list.iter().map(|index| NodeIndex::new(*index)).collect()
    }

    // `a` and `b` are both required by `t`
    fn create_shared_dependency_graph() -> ActionGraph {
        create_task_graph(
            &["a", "b", "t"],
            &[
                (2, 0, TaskDependencyType::Required),
                (2, 1, TaskDependencyType::Required),
            ],
        )
    }

    // Jobs can be marked as completed more than once — a persistent job is
    // marked when it is dispatched, and again when it exits — which must not
    // unblock a dependent that is still waiting on its other dependencies
    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_unblock_from_duplicate_completions() {
        let action_graph = create_shared_dependency_graph();
        let (context, completed) = create_job_context().await;
        let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

        let first = dispatcher.next().await.unwrap();
        let second = dispatcher.next().await.unwrap();

        assert_ne!(first, second);
        assert!(dispatcher.next().await.is_none());

        context.mark_completed(first, true).await;
        context.mark_completed(first, true).await;

        assert!(
            dispatcher.next().await.is_none(),
            "dispatched before all dependencies completed"
        );

        context.mark_completed(second, true).await;

        assert_eq!(dispatcher.next().await, Some(NodeIndex::new(2)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dispatches_dense_sync_graph() {
        for depth in [8, 12, 16, 20, 24] {
            let action_graph = create_dense_sync_graph(depth, 2);
            let (context, completed) = create_job_context().await;
            let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);
            let mut dispatched = vec![];

            while dispatcher.has_queued_jobs() {
                let Some(index) = dispatcher.next().await else {
                    break;
                };

                dispatched.push(index.index());
                context.mark_completed(index, true).await;
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
        let (context, completed) = create_job_context().await;
        let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);
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
                    Some(index) => context.mark_completed(index, true).await,
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
        let (context, completed) = create_job_context().await;
        let mut dispatcher = create_dispatcher(&action_graph, context, completed);

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

    mod wait_deps {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn releases_dependents_once_dispatched() {
            // `t` waits on `s`
            let action_graph = create_task_graph(&["s", "t"], &[(1, 0, TaskDependencyType::Wait)]);
            let (context, completed) = create_job_context().await;
            let mut dispatcher = create_dispatcher(&action_graph, context, completed);

            assert!(dispatcher.has_wait_dependents(NodeIndex::new(0)));
            assert!(!dispatcher.has_wait_dependents(NodeIndex::new(1)));

            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

            // `s` is still running, but has started
            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn releases_dependents_only_once() {
            // `t` waits on `s`, and requires `r`
            let action_graph = create_task_graph(
                &["s", "r", "t"],
                &[
                    (2, 0, TaskDependencyType::Wait),
                    (2, 1, TaskDependencyType::Required),
                ],
            );
            let (context, completed) = create_job_context().await;
            let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

            let first = dispatcher.next().await.unwrap();
            let second = dispatcher.next().await.unwrap();

            assert_eq!(
                FxHashSet::from_iter([first, second]),
                FxHashSet::from_iter(indexes(&[0, 1]))
            );

            // Completing `s` must not release `t` a second time
            context.mark_completed(NodeIndex::new(0), true).await;

            assert!(dispatcher.next().await.is_none());

            context.mark_completed(NodeIndex::new(1), true).await;

            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(2)));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn waits_for_completion_when_also_required() {
            // `t` waits on `s`, but also requires it through another edge
            let action_graph = create_task_graph(
                &["s", "t"],
                &[
                    (1, 0, TaskDependencyType::Wait),
                    (1, 0, TaskDependencyType::Required),
                ],
            );
            let (context, completed) = create_job_context().await;
            let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

            assert!(!dispatcher.has_wait_dependents(NodeIndex::new(0)));
            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));
            assert!(dispatcher.next().await.is_none());

            context.mark_completed(NodeIndex::new(0), true).await;

            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
        }
    }

    mod cleanup_deps {
        use super::*;

        /// Mark a job as completed, and whether it started, and ran its command.
        fn complete(context: &JobContext, index: usize, started: bool, executed: bool) {
            context
                .completed_queue
                .send(CompletedJob {
                    index: NodeIndex::new(index),
                    started,
                    failed: false,
                    executed,
                })
                .unwrap();
        }

        /// The job ran its command (regardless of the outcome).
        fn ran(context: &JobContext, index: usize) {
            complete(context, index, true, true);
        }

        /// The job started, but didn't run its command (skipped or cached).
        fn didnt_run(context: &JobContext, index: usize) {
            complete(context, index, true, false);
        }

        /// The job was aborted before it could start.
        fn never_started(context: &JobContext, index: usize) {
            complete(context, index, false, false);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn waits_for_parent_to_complete() {
            // `c` cleans up after `p`
            let action_graph =
                create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
            let (context, completed) = create_job_context().await;
            let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));
            assert!(dispatcher.next().await.is_none());

            ran(&context, 0);

            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn waits_for_parent_when_also_required() {
            // `c` cleans up after `p`, but also requires it
            let action_graph = create_task_graph(
                &["p", "c"],
                &[
                    (1, 0, TaskDependencyType::Required),
                    (1, 0, TaskDependencyType::Cleanup),
                ],
            );
            let (context, completed) = create_job_context().await;
            let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));
            assert!(dispatcher.next().await.is_none());

            ran(&context, 0);

            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
        }

        // Once aborted, cleanup jobs are only ran when their parents ran,
        // which is checked separately, so they must not slip through here
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_dispatch_once_aborted() {
            // `c` cleans up after `p`
            let action_graph =
                create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
            let (context, completed) = create_job_context().await;
            let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

            assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

            // Aborted before `p` could start
            context.abort_token.cancel();
            never_started(&context, 0);

            assert!(dispatcher.next().await.is_none());
            assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
        }

        mod needless {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn needed_when_parent_ran() {
                // `c` cleans up after `p`
                let action_graph =
                    create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                ran(&context, 0);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
                assert!(!dispatcher.is_needless_cleanup(NodeIndex::new(1)));
            }

            // The parent was skipped, or hydrated from the cache
            #[tokio::test(flavor = "multi_thread")]
            async fn needless_when_parent_didnt_run() {
                // `c` cleans up after `p`
                let action_graph =
                    create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                didnt_run(&context, 0);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
                assert!(dispatcher.is_needless_cleanup(NodeIndex::new(1)));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn needed_when_any_parent_ran() {
                // `c` cleans up after `p1` and `p2`
                let action_graph = create_task_graph(
                    &["p1", "p2", "c"],
                    &[
                        (2, 0, TaskDependencyType::Cleanup),
                        (2, 1, TaskDependencyType::Cleanup),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                dispatcher.next().await.unwrap();
                dispatcher.next().await.unwrap();

                ran(&context, 0);
                didnt_run(&context, 1);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(2)));
                assert!(!dispatcher.is_needless_cleanup(NodeIndex::new(2)));
            }

            // Another job requires it, so it must run regardless
            #[tokio::test(flavor = "multi_thread")]
            async fn needed_when_depended_on() {
                // `c` cleans up after `p`, and is required by `y`
                let action_graph = create_task_graph(
                    &["p", "c", "y"],
                    &[
                        (1, 0, TaskDependencyType::Cleanup),
                        (2, 1, TaskDependencyType::Required),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                didnt_run(&context, 0);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
                assert!(!dispatcher.is_needless_cleanup(NodeIndex::new(1)));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn needless_when_parent_is_a_needless_cleanup() {
                // `c2` cleans up after `c1`, which cleans up after `p`
                let action_graph = create_task_graph(
                    &["p", "c1", "c2"],
                    &[
                        (1, 0, TaskDependencyType::Cleanup),
                        (2, 1, TaskDependencyType::Cleanup),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                didnt_run(&context, 0);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
                assert!(dispatcher.is_needless_cleanup(NodeIndex::new(1)));

                // Skipped without running
                didnt_run(&context, 1);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(2)));
                assert!(dispatcher.is_needless_cleanup(NodeIndex::new(2)));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn not_a_cleanup() {
                // `t` requires `d`
                let action_graph =
                    create_task_graph(&["d", "t"], &[(1, 0, TaskDependencyType::Required)]);
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                didnt_run(&context, 0);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(1)));
                assert!(!dispatcher.is_needless_cleanup(NodeIndex::new(1)));
            }
        }

        mod after_abort {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_if_parent_ran() {
                // `c` cleans up after `p`
                let action_graph =
                    create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                ran(&context, 0);

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[1]));

                // Only taken once
                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_if_parent_ran_when_also_required() {
                // `c` cleans up after `p`, but also requires it
                let action_graph = create_task_graph(
                    &["p", "c"],
                    &[
                        (1, 0, TaskDependencyType::Required),
                        (1, 0, TaskDependencyType::Cleanup),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                // Failed, which aborted the pipeline
                context
                    .completed_queue
                    .send(CompletedJob {
                        index: NodeIndex::new(0),
                        started: true,
                        failed: true,
                        executed: true,
                    })
                    .unwrap();

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[1]));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_if_parent_didnt_run() {
                // `c` cleans up after `p`
                let action_graph =
                    create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                // Skipped, or hydrated from the cache
                didnt_run(&context, 0);

                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_if_parent_never_dispatched() {
                // `c` cleans up after `p`, which requires `d`
                let action_graph = create_task_graph(
                    &["d", "p", "c"],
                    &[
                        (1, 0, TaskDependencyType::Required),
                        (2, 1, TaskDependencyType::Cleanup),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                // Aborted while `d` was running
                ran(&context, 0);

                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
                assert!(!dispatcher.is_waiting_on_cleanups(false));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_if_parent_never_started() {
                // `c` cleans up after `p`
                let action_graph =
                    create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                // Aborted before it could start
                never_started(&context, 0);

                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn waits_for_running_parent() {
                // `c` cleans up after `p`
                let action_graph =
                    create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                // Still being terminated
                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
                assert!(dispatcher.is_waiting_on_cleanups(false));

                ran(&context, 0);

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[1]));
                assert!(dispatcher.is_waiting_on_cleanups(false));

                ran(&context, 1);

                assert!(!dispatcher.is_waiting_on_cleanups(false));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn gives_up_on_running_parent() {
                // `c` cleans up after `p`
                let action_graph =
                    create_task_graph(&["p", "c"], &[(1, 0, TaskDependencyType::Cleanup)]);
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context, completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                // Still running after it should have been terminated
                assert!(dispatcher.take_cleanups_after_abort(true).is_empty());
                assert!(!dispatcher.is_waiting_on_cleanups(true));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_if_any_parent_ran() {
                // `c` cleans up after `p1` and `p2`, where `p2` requires `d`
                let action_graph = create_task_graph(
                    &["p1", "d", "p2", "c"],
                    &[
                        (2, 1, TaskDependencyType::Required),
                        (3, 0, TaskDependencyType::Cleanup),
                        (3, 2, TaskDependencyType::Cleanup),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                let first = dispatcher.next().await.unwrap();
                let second = dispatcher.next().await.unwrap();

                assert_eq!(
                    FxHashSet::from_iter([first, second]),
                    FxHashSet::from_iter(indexes(&[0, 1]))
                );

                // Aborted after `p1` ran, so `p2` never will
                ran(&context, 0);
                ran(&context, 1);

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[3]));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_if_another_pending_parent_can_never_run() {
                // `c` cleans up after `p1` and `p2`, where `p2` cleans up
                // after `q`, which requires `d`
                let action_graph = create_task_graph(
                    &["p1", "d", "q", "p2", "c"],
                    &[
                        (2, 1, TaskDependencyType::Required),
                        (3, 2, TaskDependencyType::Cleanup),
                        (4, 0, TaskDependencyType::Cleanup),
                        (4, 3, TaskDependencyType::Cleanup),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                let first = dispatcher.next().await.unwrap();
                let second = dispatcher.next().await.unwrap();

                assert_eq!(
                    FxHashSet::from_iter([first, second]),
                    FxHashSet::from_iter(indexes(&[0, 1]))
                );

                // Aborted after `d` failed, so `q` (and `p2`) never will
                ran(&context, 0);
                ran(&context, 1);

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[4]));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_in_order() {
                // `c2` cleans up after `c1`, which cleans up after `p`
                let action_graph = create_task_graph(
                    &["p", "c1", "c2"],
                    &[
                        (1, 0, TaskDependencyType::Cleanup),
                        (2, 1, TaskDependencyType::Cleanup),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                ran(&context, 0);

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[1]));

                ran(&context, 1);

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[2]));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_waiting_cleanups_together() {
                // `c1` and `c2` clean up after `p`, and `c2` waits on `c1`
                let action_graph = create_task_graph(
                    &["p", "c1", "c2"],
                    &[
                        (1, 0, TaskDependencyType::Cleanup),
                        (2, 0, TaskDependencyType::Cleanup),
                        (2, 1, TaskDependencyType::Wait),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                assert_eq!(dispatcher.next().await, Some(NodeIndex::new(0)));

                ran(&context, 0);

                assert_eq!(
                    dispatcher.take_cleanups_after_abort(false),
                    indexes(&[1, 2])
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_one_of_the_same_action_at_a_time() {
                // `c` (twice, with different args) cleans up after `p1` and `p2`
                let action_graph = create_task_graph(
                    &["p1", "p2", "c", "c"],
                    &[
                        (2, 0, TaskDependencyType::Cleanup),
                        (3, 1, TaskDependencyType::Cleanup),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                let first = dispatcher.next().await.unwrap();
                let second = dispatcher.next().await.unwrap();

                assert_eq!(
                    FxHashSet::from_iter([first, second]),
                    FxHashSet::from_iter(indexes(&[0, 1]))
                );

                ran(&context, 0);
                ran(&context, 1);

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[2]));
                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());

                ran(&context, 2);

                assert_eq!(dispatcher.take_cleanups_after_abort(false), indexes(&[3]));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_if_own_dependency_never_started() {
                // `c` cleans up after `p`, but requires `d` itself
                let action_graph = create_task_graph(
                    &["p", "d", "c"],
                    &[
                        (2, 0, TaskDependencyType::Cleanup),
                        (2, 1, TaskDependencyType::Required),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                let first = dispatcher.next().await.unwrap();
                let second = dispatcher.next().await.unwrap();

                assert_eq!(
                    FxHashSet::from_iter([first, second]),
                    FxHashSet::from_iter(indexes(&[0, 1]))
                );

                // Aborted before `d` could start
                ran(&context, 0);
                never_started(&context, 1);

                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_if_own_non_task_dependency_failed() {
                // `c` cleans up after `p`, but requires `sync` itself
                let action_graph = create_task_graph(
                    &["p", "sync", "c"],
                    &[
                        (2, 0, TaskDependencyType::Cleanup),
                        (2, 1, TaskDependencyType::Required),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                let first = dispatcher.next().await.unwrap();
                let second = dispatcher.next().await.unwrap();

                assert_eq!(
                    FxHashSet::from_iter([first, second]),
                    FxHashSet::from_iter(indexes(&[0, 1]))
                );

                // Aborted because syncing failed
                ran(&context, 0);
                context
                    .completed_queue
                    .send(CompletedJob {
                        index: NodeIndex::new(1),
                        started: true,
                        failed: true,
                        executed: false,
                    })
                    .unwrap();

                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_if_wait_dependency_never_started() {
                // `c` cleans up after `p`, but waits on `s` itself
                let action_graph = create_task_graph(
                    &["p", "s", "c"],
                    &[
                        (2, 0, TaskDependencyType::Cleanup),
                        (2, 1, TaskDependencyType::Wait),
                    ],
                );
                let (context, completed) = create_job_context().await;
                let mut dispatcher = create_dispatcher(&action_graph, context.clone(), completed);

                let first = dispatcher.next().await.unwrap();
                let second = dispatcher.next().await.unwrap();

                assert_eq!(
                    FxHashSet::from_iter([first, second]),
                    FxHashSet::from_iter(indexes(&[0, 1]))
                );

                // Aborted before `s` could start
                ran(&context, 0);
                never_started(&context, 1);

                assert!(dispatcher.take_cleanups_after_abort(false).is_empty());
            }
        }
    }
}
