use crate::action_graph_error::ActionGraphError;
use crate::action_node::ActionNode;
use moon_common::is_test_env;
use petgraph::dot::{Config, Dot};
use petgraph::prelude::*;
use petgraph::visit::{IntoEdgeReferences, IntoNodeReferences};
use rustc_hash::{FxHashMap, FxHashSet};
use tracing::{debug, trace};

pub type GraphType = DiGraph<ActionNode, ()>;
pub type IndicesMap = FxHashMap<ActionNode, NodeIndex>;

pub struct ActionGraph {
    graph: GraphType,
    indices: IndicesMap,
}

impl ActionGraph {
    pub fn new(graph: GraphType, indices: IndicesMap) -> Self {
        debug!("Creating action graph");

        ActionGraph { graph, indices }
    }

    pub fn try_iter(&self) -> miette::Result<ActionGraphIter> {
        ActionGraphIter::new(&self.graph)
    }

    pub fn is_empty(&self) -> bool {
        self.get_node_count() == 0
    }

    pub fn get_index_from_node(&self, node: &ActionNode) -> Option<&NodeIndex> {
        self.indices.get(node)
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

#[derive(Debug)]
pub struct ActionGraphIter<'graph> {
    graph: &'graph GraphType,
    indices: Vec<NodeIndex>,
    visited: FxHashSet<NodeIndex>,
    completed: FxHashSet<NodeIndex>,
}

impl<'graph> ActionGraphIter<'graph> {
    pub fn new(graph: &'graph GraphType) -> miette::Result<Self> {
        match petgraph::algo::toposort(graph, None) {
            Ok(indices) => {
                debug!(
                    order = ?indices.iter().map(|i| i.index()).collect::<Vec<_>>(),
                    "Creating topological iterator for action graph",
                );

                Ok(Self {
                    graph,
                    indices,
                    visited: FxHashSet::default(),
                    completed: FxHashSet::default(),
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
        self.completed.len() < self.graph.node_count()
    }

    pub fn mark_completed(&mut self, index: usize) {
        let index = NodeIndex::new(index);

        if !self.completed.contains(&index) {
            trace!(index = index.index(), "Marking action as complete");

            self.completed.insert(index);
        }
    }
}

// This is based on the `Topo` struct from petgraph!
impl<'graph> Iterator for ActionGraphIter<'graph> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        for idx in &self.indices {
            if self.visited.contains(idx) || self.completed.contains(idx) {
                continue;
            }

            // Ensure all dependencies of the index have completed
            if self
                .graph
                .neighbors_directed(*idx, Direction::Outgoing)
                .all(|dep| self.completed.contains(&dep))
            {
                self.visited.insert(*idx);

                trace!(
                    index = idx.index(),
                    "Action ready and dependencies have been met, running",
                );

                return Some(*idx);
            }
        }

        None
    }
}
