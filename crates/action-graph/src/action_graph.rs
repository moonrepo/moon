use crate::action_graph_error::ActionGraphError;
use daggy::Dag;
use graph_cycles::Cycles;
use moon_action::ActionNode;
use moon_config::TaskDependencyType;
use moon_graph_utils::*;
use petgraph::prelude::*;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use tracing::debug;

pub type ActionGraphType = Dag<NodeIndex, TaskDependencyType>;

pub struct ActionGraph {
    graph: ActionGraphType,
    nodes: FxHashMap<NodeIndex, ActionNode>,
}

impl ActionGraph {
    pub fn new(graph: ActionGraphType, nodes: FxHashMap<NodeIndex, ActionNode>) -> Self {
        debug!("Creating action graph");

        ActionGraph { graph, nodes }
    }

    pub fn is_empty(&self) -> bool {
        self.get_node_count() == 0
    }

    pub fn get_inner_graph(&self) -> &ActionGraphType {
        &self.graph
    }

    pub fn get_inner_nodes(&self) -> &FxHashMap<NodeIndex, ActionNode> {
        &self.nodes
    }

    pub fn get_node_from_index(&self, index: &NodeIndex) -> Option<&ActionNode> {
        self.nodes.get(index)
    }

    pub fn get_nodes(&self) -> Vec<&ActionNode> {
        self.nodes.values().collect()
    }

    pub fn get_node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn group_priorities(&self, topo_indices: Vec<NodeIndex>) -> BTreeMap<u8, Vec<NodeIndex>> {
        let mut groups = BTreeMap::default();

        // These are purely for debugging
        let mut critical = vec![];
        let mut high = vec![];
        let mut normal = vec![];
        let mut low = vec![];

        for node_index in topo_indices {
            let node = self.get_node_by_index(&node_index);
            let priority = node.get_priority();

            match priority {
                0 => critical.push(node_index),
                1 => high.push(node_index),
                2 => normal.push(node_index),
                3 => low.push(node_index),
                _ => {}
            };

            groups
                .entry(priority)
                .or_insert_with(Vec::new)
                .push(node_index);
        }

        debug!(
            critical = ?critical,
            high = ?high,
            normal = ?normal,
            low = ?low,
            "Grouping action graph based on priority",
        );

        groups
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
                            .map(|n| self.get_node_by_index(n).label())
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
                    .map(|n| self.get_node_by_index(n).label())
                    .unwrap_or_else(|| "(unknown)".into()),
            )
            .into()),
        }
    }
}

impl GraphData<ActionNode, TaskDependencyType, String> for ActionGraph {
    fn get_graph(&self) -> &DiGraph<NodeIndex, TaskDependencyType> {
        self.graph.graph()
    }

    fn get_nodes(&self) -> FxHashMap<NodeIndex, &ActionNode> {
        self.nodes
            .iter()
            .map(|(index, node)| (*index, node))
            .collect()
    }

    fn get_node_by_index(&self, index: &NodeIndex) -> &ActionNode {
        &self.nodes[index]
    }

    fn get_node_key(&self, node: &ActionNode) -> String {
        node.label()
    }
}

impl GraphConnections<ActionNode, TaskDependencyType, String> for ActionGraph {
    fn get_node_index(&self, node: &ActionNode) -> NodeIndex {
        for (index, n) in &self.nodes {
            if n == node {
                return *index;
            }
        }

        panic!("Action node not found in graph!");
    }
}

impl GraphConversions<ActionNode, TaskDependencyType, String> for ActionGraph {}

impl GraphToDot<ActionNode, TaskDependencyType, String> for ActionGraph {}

impl GraphToJson<ActionNode, TaskDependencyType, String> for ActionGraph {}
