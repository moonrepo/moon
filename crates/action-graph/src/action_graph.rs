use crate::action_graph_error::ActionGraphError;
use graph_cycles::Cycles;
use moon_action::ActionNode;
use moon_config::TaskDependencyType;
use moon_graph_utils::*;
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use tracing::debug;

pub type ActionGraphType = DiGraph<ActionNode, TaskDependencyType>;

pub struct ActionGraph {
    graph: ActionGraphType,

    // Indexes of every action that runs *after* another action as a cleanup.
    // Tracked separately so that they can be located without having to scan
    // every edge in the graph.
    cleanup_indices: FxHashSet<NodeIndex>,

    // Index of every node, for looking up a node's index from the node itself.
    // Only the graph traits need it, so it's created on first use.
    node_indices: OnceLock<FxHashMap<ActionNode, NodeIndex>>,
}

impl ActionGraph {
    pub fn new(graph: ActionGraphType) -> Self {
        Self::new_with_cleanups(graph, FxHashSet::default())
    }

    pub fn new_with_cleanups(
        graph: ActionGraphType,
        cleanup_indices: FxHashSet<NodeIndex>,
    ) -> Self {
        debug!("Creating action graph");

        ActionGraph {
            graph,
            cleanup_indices,
            node_indices: OnceLock::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.get_node_count() == 0
    }

    pub fn get_cleanup_indices(&self) -> &FxHashSet<NodeIndex> {
        &self.cleanup_indices
    }

    pub fn is_cleanup_index(&self, index: &NodeIndex) -> bool {
        self.cleanup_indices.contains(index)
    }

    pub fn get_inner_graph(&self) -> &ActionGraphType {
        &self.graph
    }

    pub fn get_node_from_index(&self, index: &NodeIndex) -> Option<&ActionNode> {
        self.graph.node_weight(*index)
    }

    pub fn get_nodes(&self) -> Vec<&ActionNode> {
        self.graph.node_weights().collect()
    }

    pub fn get_node_count(&self) -> usize {
        self.graph.node_count()
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
                0 => critical.push(node_index.index()),
                1 => high.push(node_index.index()),
                2 => normal.push(node_index.index()),
                3 => low.push(node_index.index()),
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
                            .map(|node| node.label())
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
                    .map(|node| node.label())
                    .unwrap_or_else(|| "(unknown)".into()),
            )
            .into()),
        }
    }
}

impl GraphData<ActionNode, TaskDependencyType, String> for ActionGraph {
    type NodeWeight = ActionNode;

    fn get_graph(&self) -> &ActionGraphType {
        &self.graph
    }

    fn get_nodes(&self) -> FxHashMap<NodeIndex, &ActionNode> {
        self.graph
            .node_indices()
            .map(|index| (index, &self.graph[index]))
            .collect()
    }

    fn get_node_by_index(&self, index: &NodeIndex) -> &ActionNode {
        &self.graph[*index]
    }

    fn get_node_key(&self, node: &ActionNode) -> String {
        node.label()
    }
}

impl GraphConnections<ActionNode, TaskDependencyType, String> for ActionGraph {
    fn get_node_index(&self, node: &ActionNode) -> NodeIndex {
        let indices = self.node_indices.get_or_init(|| {
            let mut indices = FxHashMap::default();

            for index in self.graph.node_indices() {
                indices.entry(self.graph[index].clone()).or_insert(index);
            }

            indices
        });

        *indices.get(node).expect("Action node not found in graph!")
    }
}

impl GraphConversions<ActionNode, TaskDependencyType, String> for ActionGraph {}

impl GraphToDot<ActionNode, TaskDependencyType, String> for ActionGraph {}

impl GraphToJson<ActionNode, TaskDependencyType, String> for ActionGraph {}
