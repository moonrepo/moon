use crate::job_context::JobContext;
use moon_action_graph::{ActionGraph, GraphType};
use petgraph::prelude::*;
use rustc_hash::FxHashSet;
use tracing::trace;

pub struct JobDispatcher<'graph> {
    context: JobContext,
    graph: &'graph GraphType,
    indices: Vec<NodeIndex>,
    visited: FxHashSet<NodeIndex>,
}

impl<'graph> JobDispatcher<'graph> {
    pub fn new(
        action_graph: &'graph ActionGraph,
        context: JobContext,
        indices: Vec<NodeIndex>,
    ) -> Self {
        Self {
            context,
            graph: action_graph.get_inner_graph(),
            indices,
            visited: FxHashSet::default(),
        }
    }

    pub fn has_queued_jobs(&self) -> bool {
        self.visited.len() < self.graph.node_count()
    }
}

// This is based on the `Topo` struct from petgraph!
impl<'graph> JobDispatcher<'graph> {
    pub async fn next(&mut self) -> Option<NodeIndex> {
        let completed = self.context.completed_jobs.read().await;

        for idx in &self.indices {
            if self.visited.contains(idx) || completed.contains(idx) {
                continue;
            }

            // Ensure all dependencies of the index have completed
            let mut deps = vec![];

            if self
                .graph
                .neighbors_directed(*idx, Direction::Outgoing)
                .all(|dep| {
                    deps.push(dep.index());
                    completed.contains(&dep)
                })
            {
                if let Some(node) = self.graph.node_weight(*idx) {
                    let label = node.label();

                    // If the same exact action is currently running,
                    // avoid running another in parallel to avoid weird
                    // collisions. This is especially true for `RunTask`,
                    // where different args/env vars run the same task,
                    // but with slightly different variance.
                    {
                        if node.is_standard()
                            && self
                                .context
                                .running_jobs
                                .read()
                                .await
                                .values()
                                .any(|running_label| running_label == &label)
                        {
                            continue;
                        }
                    }

                    self.context.running_jobs.write().await.insert(*idx, label);
                }

                trace!(
                    index = idx.index(),
                    deps = ?deps,
                    "Dispatching job",
                );

                self.visited.insert(*idx);

                return Some(*idx);
            }
        }

        None
    }
}
