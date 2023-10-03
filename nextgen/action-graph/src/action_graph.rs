use crate::action_graph_error::ActionGraphError;
use crate::action_node::ActionNode;
use petgraph::dot::{Config, Dot};
use petgraph::prelude::*;
use rustc_hash::FxHashMap;

pub struct ActionGraph {
    graph: StableGraph<ActionNode, ()>,
    indices: FxHashMap<ActionNode, NodeIndex>,
}

impl ActionGraph {
    pub fn new(
        graph: StableGraph<ActionNode, ()>,
        indices: FxHashMap<ActionNode, NodeIndex>,
    ) -> Self {
        ActionGraph { graph, indices }
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

    pub fn create_queue(&self) -> miette::Result<()> {
        self.detect_cycle()?;

        Ok(())
    }

    pub fn to_dot(&self) -> String {
        let graph = self.graph.map(|_, n| n.label(), |_, e| e);

        let dot = Dot::with_attr_getters(
            &graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, e| {
                if e.source().index() == 0 {
                    String::from("arrowhead=none")
                } else {
                    String::from("arrowhead=box, arrowtail=box")
                }
            },
            &|_, n| {
                format!(
                    "label=\"{}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black",
                    n.1
                )
            },
        );

        format!("{dot:?}")
    }

    pub fn to_labeled_graph(&self) -> StableGraph<String, String> {
        self.graph.map(|_, n| n.label(), |_, _| String::new())
    }

    fn detect_cycle(&self) -> miette::Result<()> {
        let scc = petgraph::algo::kosaraju_scc(&self.graph);

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
