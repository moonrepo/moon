use crate::job_context::JobContext;
use moon_action_graph::{ActionGraph, ActionGraphType};
use petgraph::prelude::*;
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;
use tracing::trace;

pub struct JobDispatcher<'graph> {
    context: JobContext,
    graph: &'graph ActionGraphType,
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
    ) -> Option<NodeIndex> {
        if self.visited.contains(&index) || completed.contains(&index) {
            return None;
        }

        // Ensure all dependencies of the index have
        // completed before dispatching
        if self
            .graph
            .neighbors_directed(index, Direction::Outgoing)
            .all(|dep_index| completed.contains(&dep_index))
        {
            return Some(index);
        }

        // If not all dependencies have completed yet,
        // attempt to find a dependency to run
        if group < 2 {
            for dep_index in self.graph.neighbors_directed(index, Direction::Outgoing) {
                if let Some(index) = self.find_applicable_index(group, dep_index, completed) {
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
        let completed = self.context.completed_jobs.read().await;

        // Loop based on priority groups, from critical to low
        {
            for (group, indices) in &self.groups {
                // Then loop through the indices within the group,
                // which are topologically sorted
                for maybe_index in indices {
                    let Some(index) = self.find_applicable_index(*group, *maybe_index, &completed)
                    else {
                        continue;
                    };

                    if let Some(node) = self.graph.node_weight(index) {
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
