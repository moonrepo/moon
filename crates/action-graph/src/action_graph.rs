use crate::action_graph_error::ActionGraphError;
use graph_cycles::Cycles;
use moon_action::ActionNode;
use moon_config::TaskDependencyType;
use moon_graph_utils::*;
use moon_layout_graph::LayoutGraph;
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};
use tracing::debug;

pub type ActionGraphType = DiGraph<ActionNode, TaskDependencyType>;

pub struct ActionGraph {
    graph: ActionGraphType,

    // Indexes of every action that runs *after* another action as a cleanup.
    // Tracked separately so that they can be located without having to scan
    // every edge in the graph.
    cleanup_indices: FxHashSet<NodeIndex>,

    // Where the toolchains of the graph's actions were placed (their
    // dependencies roots), which actions read while running.
    layout_graph: Arc<LayoutGraph>,

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
            layout_graph: Arc::new(LayoutGraph::default()),
            node_indices: OnceLock::new(),
        }
    }

    /// Set the layout graph that the graph's toolchains were placed in.
    pub fn with_layout_graph(mut self, layout_graph: Arc<LayoutGraph>) -> Self {
        self.layout_graph = layout_graph;
        self
    }

    pub fn get_layout_graph(&self) -> &Arc<LayoutGraph> {
        &self.layout_graph
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
        // The builder never links an edge that closes a cycle, so sort first,
        // and only search for the cycle to report when sorting fails
        match petgraph::algo::toposort(&self.graph, None) {
            Ok(mut indices) => {
                indices.reverse();

                debug!(
                    order = ?indices.iter().map(|i| i.index()).collect::<Vec<_>>(),
                    "Sorting action graph topologically",
                );

                Ok(indices)
            }
            Err(cycle) => {
                let mut indices: Vec<NodeIndex> = vec![];

                self.graph.visit_cycles(|_, found| {
                    indices.extend(found);
                    std::ops::ControlFlow::Break(())
                });

                // Fallback to the node that the sort detected the cycle at
                if indices.is_empty() {
                    indices.push(cycle.node_id());
                }

                Err(ActionGraphError::CycleDetected(
                    indices
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
                .into())
            }
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
