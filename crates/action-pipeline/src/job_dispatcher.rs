use crate::job_context::JobContext;
use moon_action::ActionNode;
use moon_action_graph::{ActionGraph, ActionGraphType};
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use tracing::trace;

pub struct JobDispatcher<'graph> {
    context: JobContext,
    graph: &'graph ActionGraphType,
    nodes: &'graph FxHashMap<NodeIndex, ActionNode>,
    groups: BTreeMap<u8, Vec<NodeIndex>>, // topo
    visited: FxHashSet<NodeIndex>,
}

impl<'graph> JobDispatcher<'graph> {
    pub fn new(
        action_graph: &'graph ActionGraph,
        context: JobContext,
        groups: BTreeMap<u8, Vec<NodeIndex>>,
    ) -> Self {
        Self {
            context,
            graph: action_graph.get_inner_graph(),
            nodes: action_graph.get_inner_nodes(),
            groups,
            visited: FxHashSet::default(),
        }
    }

    pub fn has_queued_jobs(&self) -> bool {
        self.visited.len() < self.graph.node_count()
    }

    pub fn find_applicable_index(
        &self,
        group: u8,
        index: NodeIndex,
        completed: &FxHashSet<NodeIndex>,
        traversed: &mut FxHashSet<NodeIndex>,
    ) -> Option<NodeIndex> {
        if !traversed.insert(index) || self.visited.contains(&index) || completed.contains(&index) {
            return None;
        }

        // Ensure all dependencies of the index have
        // completed before dispatching
        if self
            .graph
            .graph()
            .neighbors_directed(index, Direction::Outgoing)
            .all(|dep_index| completed.contains(&dep_index))
        {
            return Some(index);
        }

        // If not all dependencies have completed yet,
        // attempt to find a dependency to run
        if group < 2 {
            for dep_index in self
                .graph
                .graph()
                .neighbors_directed(index, Direction::Outgoing)
            {
                if let Some(index) =
                    self.find_applicable_index(group, dep_index, completed, traversed)
                {
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
        let completed = self.context.completed_jobs.read().await.clone();
        // Avoid repeatedly traversing the same blocked dependency subgraph
        // while a prerequisite action is still running.
        let mut traversed = FxHashSet::default();

        // Loop based on priority groups, from critical to low
        {
            for (group, indices) in &self.groups {
                // Then loop through the indices within the group,
                // which are topologically sorted
                for maybe_index in indices {
                    let Some(index) = self.find_applicable_index(
                        *group,
                        *maybe_index,
                        &completed,
                        &mut traversed,
                    ) else {
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
                                trace!(
                                    index = index.index(),
                                    running_index = running_index.0.index(),
                                    "Another job of a similar type is currently running, deferring dispatch",
                                );

                                continue;
                            }

                            self.context.running_jobs.write().await.insert(index, id);
                        }
                    }

                    trace!(index = index.index(), "Dispatching job");

                    self.visited.insert(index);

                    return Some(index);
                }
            }
        }

        // Remove indices and groups once they have been completed
        {
            self.groups.retain(|_, indices| {
                indices.retain(|index| !completed.contains(index));

                !indices.is_empty()
            });
        }

        None
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

    async fn create_job_context() -> JobContext {
        let (sender, _receiver) = mpsc::channel::<Action>(8);

        JobContext {
            abort_token: CancellationToken::new(),
            cancel_token: CancellationToken::new(),
            completed_jobs: Arc::new(RwLock::new(FxHashSet::default())),
            emitter: Arc::new(EventEmitter::default()),
            result_sender: sender,
            running_jobs: Arc::new(RwLock::new(FxHashMap::default())),
            semaphore: Arc::new(Semaphore::new(1)),
            workspace_graph: Arc::new(WorkspaceGraph::default()),
        }
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

    #[tokio::test(flavor = "multi_thread")]
    async fn dispatches_dense_sync_graph() {
        for depth in [8, 12, 16, 20, 24] {
            let action_graph = create_dense_sync_graph(depth, 2);
            let groups = action_graph.group_priorities(action_graph.sort_topological().unwrap());
            let context = create_job_context().await;
            let mut dispatcher = JobDispatcher::new(&action_graph, context.clone(), groups);
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

    #[tokio::test(flavor = "multi_thread")]
    async fn avoids_rewalking_blocked_sync_subgraphs() {
        let action_graph = create_dense_sync_graph(12, 4);
        let groups = action_graph.group_priorities(action_graph.sort_topological().unwrap());
        let context = create_job_context().await;
        let mut dispatcher = JobDispatcher::new(&action_graph, context, groups);

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
