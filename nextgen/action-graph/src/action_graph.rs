use crate::action_graph_error::ActionGraphError;
use crate::action_node::ActionNode;
use moon_common::is_test_env;
use petgraph::dot::{Config, Dot};
use petgraph::prelude::*;
use petgraph::visit::{IntoEdgeReferences, IntoNodeReferences};
use rustc_hash::FxHashSet;
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

    pub fn try_iter(&self) -> miette::Result<ActionGraphIter> {
        ActionGraphIter::new(&self.graph)
    }

    pub fn is_empty(&self) -> bool {
        self.get_node_count() == 0
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
    graph: &'graph GraphType,
    indices: Vec<NodeIndex>,
    visited: FxHashSet<NodeIndex>,
    completed: Arc<RwLock<FxHashSet<NodeIndex>>>,

    pub receiver: Option<mpsc::Receiver<usize>>,
    pub sender: mpsc::Sender<usize>,
}

impl<'graph> ActionGraphIter<'graph> {
    pub fn new(graph: &'graph GraphType) -> miette::Result<Self> {
        match petgraph::algo::toposort(graph, None) {
            Ok(mut indices) => {
                indices.reverse();

                debug!(
                    order = ?indices.iter().map(|i| i.index()).collect::<Vec<_>>(),
                    "Iterating action graph topologically",
                );

                let (sender, receiver) = mpsc::channel();

                Ok(Self {
                    graph,
                    indices,
                    visited: FxHashSet::default(),
                    completed: Arc::new(RwLock::new(FxHashSet::default())),
                    receiver: Some(receiver),
                    sender,
                })
            }
            Err(cycle) => Err(ActionGraphError::CycleDetected(
                graph
                    .node_weight(cycle.node_id())
                    .map(|n| n.label())
                    .unwrap_or_else(|| "(unknown)".into()),
            )
            .into()),
        }
    }

    pub fn has_pending(&self) -> bool {
        self.completed.read().unwrap().len() < self.graph.node_count()
    }

    pub fn mark_completed(&mut self, index: NodeIndex) {
        self.completed.write().unwrap().insert(index);
    }

    pub fn monitor_completed(&mut self) {
        let completed = Arc::clone(&self.completed);
        let receiver = self.receiver.take().unwrap();

        spawn(move || {
            while let Ok(idx) = receiver.recv() {
                let index = NodeIndex::new(idx);

                trace!(index = index.index(), "Marking action as complete");

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
                self.visited.insert(*idx);

                trace!(
                    index = idx.index(),
                    deps = ?deps,
                    "Running action",
                );

                return Some(*idx);
            }
        }

        None
    }
}
