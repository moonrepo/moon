use crate::action_graph_error::ActionGraphError;
use crate::action_node::ActionNode;
use moon_common::is_test_env;
use petgraph::dot::{Config, Dot};
use petgraph::prelude::*;
use petgraph::visit::{IntoEdgeReferences, IntoNodeReferences};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

pub type GraphType = DiGraph<ActionNode, ()>;
pub type IndicesMap = FxHashMap<ActionNode, NodeIndex>;

pub struct ActionGraph {
    graph: GraphType,
    indices: IndicesMap,

    // States when iterating
    queue: VecDeque<NodeIndex>,
    visited: FxHashSet<NodeIndex>,
}

impl ActionGraph {
    pub fn new(graph: GraphType, indices: IndicesMap) -> Self {
        ActionGraph {
            graph,
            indices,
            queue: VecDeque::default(),
            visited: FxHashSet::default(),
        }
    }

    pub fn reset_iterator(&mut self) -> miette::Result<()> {
        // self.detect_cycle()?;

        self.queue.clear();
        self.visited.clear();

        // Extract root/initial nodes (those without edges)
        self.queue.extend(self.graph.node_indices().filter(|&idx| {
            self.graph
                .neighbors_directed(idx, Outgoing)
                .next()
                .is_none()
        }));

        Ok(())
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

    pub fn to_labeled_graph(&self) -> DiGraph<String, String> {
        self.graph.map(|_, n| n.label(), |_, _| String::new())
    }

    fn detect_cycle(&self) -> miette::Result<()> {
        if self.is_empty() || self.get_node_count() == 1 {
            return Ok(());
        }

        let scc = petgraph::algo::kosaraju_scc(&self.graph);

        // TODO
        dbg!(&scc);

        // The cycle is always the last sequence in the list
        let Some(cycle) = scc.last() else {
            return Err(ActionGraphError::CycleDetected("(unknown)".into()).into());
        };

        let path = cycle
            .iter()
            .filter_map(|i| self.get_node_from_index(i).map(|n| n.label()))
            .collect::<Vec<String>>()
            .join(" → ");

        Err(ActionGraphError::CycleDetected(path).into())
    }
}

// This is based on the `Topo` struct from petgraph!
impl Iterator for ActionGraph {
    type Item = ActionNode;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(idx) = self.queue.pop_front() {
            if self.visited.contains(&idx) {
                continue;
            }

            self.visited.insert(idx);

            for neighbor in self.graph.neighbors_directed(idx, Direction::Incoming) {
                // Look at each neighbor, and those that only have incoming edges
                // from the already ordered list, they are the next to visit.
                if self
                    .graph
                    .neighbors_directed(neighbor, Direction::Outgoing)
                    .all(|b| self.visited.contains(&b))
                {
                    self.queue.push_back(neighbor);
                }
            }

            return self.graph.node_weight(idx).map(|n| n.to_owned());
        }

        None
    }
}
