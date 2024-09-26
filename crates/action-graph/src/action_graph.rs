use crate::action_graph_error::ActionGraphError;
use graph_cycles::Cycles;
use moon_action::ActionNode;
use moon_common::{color, is_test_env};
use petgraph::dot::{Config, Dot};
use petgraph::prelude::*;
use petgraph::visit::{IntoEdgeReferences, IntoNodeReferences};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::{mpsc, Arc, RwLock};
use std::thread::spawn;
use tracing::{debug, trace};

pub type GraphType = DiGraph<ActionNode, ()>;

pub struct ActionGraph {
    graph: GraphType,
}

impl ActionGraph {
    pub fn new(graph: GraphType) -> Self {
        debug!("Creating action graph");

        ActionGraph { graph }
    }

    pub fn create_iter(&self, indices: Vec<NodeIndex>) -> ActionGraphIter {
        ActionGraphIter::new(&self.graph, indices)
    }

    pub fn is_empty(&self) -> bool {
        self.get_node_count() == 0
    }

    pub fn get_inner_graph(&self) -> &GraphType {
        &self.graph
    }

    pub fn get_nodes(&self) -> Vec<&ActionNode> {
        self.graph.node_weights().collect()
    }

    pub fn get_node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn get_node_from_index(&self, index: &NodeIndex) -> Option<&ActionNode> {
        self.graph.node_weight(*index)
    }

    pub fn labeled_graph(&self) -> DiGraph<String, String> {
        self.graph.map(|_, n| n.label(), |_, _| String::new())
    }

    pub fn sort_topological(&self) -> miette::Result<Vec<NodeIndex>> {
        // Detect any cycles first
        let mut cycle: Vec<NodeIndex> = vec![];

        self.graph.visit_cycles(|_, c| {
            cycle.extend(c);
            std::ops::ControlFlow::Break(())
        });

        if !cycle.is_empty() {
            return Err(ActionGraphError::CycleDetected(
                cycle
                    .into_iter()
                    .map(|index| {
                        self.graph
                            .node_weight(index)
                            .map(|n| n.label())
                            .unwrap_or_else(|| "(unknown)".into())
                    })
                    .collect::<Vec<_>>()
                    .join(" → "),
            )
            .into());
        }

        // Then sort topologically
        match petgraph::algo::toposort(&self.graph, None) {
            Ok(mut indices) => {
                indices.reverse();

                debug!(
                    order = ?indices.iter().map(|i| i.index()).collect::<Vec<_>>(),
                    "Sorting action graph topologically",
                );

                Ok(indices)
            }
            // For some reason the topo sort can detect a cycle,
            // that wasn't previously detected, so error...
            Err(cycle) => Err(ActionGraphError::CycleDetected(
                self.graph
                    .node_weight(cycle.node_id())
                    .map(|n| n.label())
                    .unwrap_or_else(|| "(unknown)".into()),
            )
            .into()),
        }
    }

    pub fn to_dot(&self) -> String {
        type DotGraph = DiGraph<String, ()>;

        let is_test = is_test_env() || cfg!(debug_assertions);
        let graph = self.graph.map(|_, n| n.label(), |_, _| ());

        let edge = |_: &DotGraph, e: <&DotGraph as IntoEdgeReferences>::EdgeRef| {
            if is_test {
                String::new()
            } else if e.source().index() == 0 {
                String::from("arrowhead=none")
            } else {
                String::from("arrowhead=box, arrowtail=box")
            }
        };

        let node = |_: &DotGraph, n: <&DotGraph as IntoNodeReferences>::NodeRef| {
            if is_test {
                format!("label=\"{}\" ", n.1)
            } else {
                format!(
                    "label=\"{}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black ",
                    n.1
                )
            }
        };

        let dot = Dot::with_attr_getters(
            &graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &edge,
            &node,
        );

        format!("{dot:?}")
    }
}

pub struct ActionGraphIter<'graph> {
    completed: Arc<RwLock<FxHashSet<NodeIndex>>>,
    graph: &'graph GraphType,
    indices: Vec<NodeIndex>,
    running: Arc<RwLock<FxHashMap<NodeIndex, String>>>,
    visited: FxHashSet<NodeIndex>,

    pub receiver: Option<mpsc::Receiver<usize>>,
    pub sender: mpsc::Sender<usize>,
}

impl<'graph> ActionGraphIter<'graph> {
    pub fn new(graph: &'graph GraphType, indices: Vec<NodeIndex>) -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            completed: Arc::new(RwLock::new(FxHashSet::default())),
            graph,
            indices,
            receiver: Some(receiver),
            running: Arc::new(RwLock::new(FxHashMap::default())),
            sender,
            visited: FxHashSet::default(),
        }
    }

    pub fn has_pending(&self) -> bool {
        self.completed.read().unwrap().len() < self.graph.node_count()
    }

    pub fn mark_completed(&mut self, index: NodeIndex) {
        self.running.write().unwrap().remove(&index);
        self.completed.write().unwrap().insert(index);
    }

    pub fn monitor_completed(&mut self) {
        let completed = Arc::clone(&self.completed);
        let running = Arc::clone(&self.running);
        let receiver = self.receiver.take().unwrap();

        spawn(move || {
            while let Ok(idx) = receiver.recv() {
                let index = NodeIndex::new(idx);

                trace!(index = index.index(), "Marking action as complete");

                running.write().unwrap().remove(&index);
                completed.write().unwrap().insert(index);
            }
        });
    }
}

// This is based on the `Topo` struct from petgraph!
impl<'graph> Iterator for ActionGraphIter<'graph> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let completed = self.completed.read().unwrap();

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
                                .running
                                .read()
                                .unwrap()
                                .values()
                                .any(|running_label| running_label == &label)
                        {
                            continue;
                        }
                    }

                    trace!(
                        index = idx.index(),
                        deps = ?deps,
                        "Enqueuing action {}",
                        color::muted_light(&label),
                    );

                    self.running.write().unwrap().insert(*idx, label);
                } else {
                    trace!(
                        index = idx.index(),
                        deps = ?deps,
                        "Enqueuing action",
                    );
                }

                self.visited.insert(*idx);

                return Some(*idx);
            }
        }

        None
    }
}
