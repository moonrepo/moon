use crate::action_graph_error::ActionGraphError;
use graph_cycles::Cycles;
use moon_action::ActionNode;
use moon_config::TaskDependencyType;
use moon_graph_utils::*;
use petgraph::prelude::*;
use std::collections::BTreeMap;
use tracing::debug;

pub type ActionGraphType = DiGraph<ActionNode, TaskDependencyType>;

pub struct ActionGraph {
    graph: ActionGraphType,
}

impl ActionGraph {
    pub fn new(graph: ActionGraphType) -> Self {
        debug!("Creating action graph");

        ActionGraph { graph }
    }

    pub fn is_empty(&self) -> bool {
        self.get_node_count() == 0
    }

    pub fn get_inner_graph(&self) -> &ActionGraphType {
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

    pub fn group_priorities(&self, topo_indices: Vec<NodeIndex>) -> BTreeMap<u8, Vec<NodeIndex>> {
        let mut groups = BTreeMap::default();

        // These are purely for debugging
        let mut critical = vec![];
        let mut high = vec![];
        let mut normal = vec![];
        let mut low = vec![];

        for index in topo_indices {
            let node = self.graph.node_weight(index).unwrap();
            let node_index = index.index();
            let priority = node.get_priority();

            match priority {
                0 => critical.push(node_index),
                1 => high.push(node_index),
                2 => normal.push(node_index),
                3 => low.push(node_index),
                _ => {}
            };

            groups.entry(priority).or_insert_with(Vec::new).push(index);
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
}

impl GraphData<ActionNode, TaskDependencyType, String> for ActionGraph {
    fn get_graph(&self) -> &DiGraph<ActionNode, TaskDependencyType> {
        &self.graph
    }

    fn get_node_key(&self, node: &ActionNode) -> String {
        node.label()
    }
}

impl GraphToDot<ActionNode, TaskDependencyType, String> for ActionGraph {}

impl GraphToJson<ActionNode, TaskDependencyType, String> for ActionGraph {}
