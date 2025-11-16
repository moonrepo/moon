use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use rustc_hash::{FxHashMap, FxHashSet};

/// Represents a partitioned subgraph with its nodes and edges
#[derive(Debug, Clone)]
pub struct Partition<N, E> {
    /// The nodes in this partition
    pub nodes: Vec<N>,
    /// The subgraph for this partition
    pub graph: DiGraph<N, E>,
    /// Partition index
    pub index: usize,
}

/// A DAG that can be partitioned into multiple subgraphs based on node count
#[derive(Debug, Clone)]
pub struct PartitionableDAG<N, E> {
    graph: DiGraph<N, E>,
}

impl<N: Clone, E: Clone> PartitionableDAG<N, E> {
    /// Creates a new empty partitionable DAG
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
        }
    }

    /// Creates a new partitionable DAG from an existing graph
    pub fn from_graph(graph: DiGraph<N, E>) -> Result<Self, String> {
        // Verify it's acyclic
        if toposort(&graph, None).is_err() {
            return Err("Graph contains cycles and is not a DAG".to_string());
        }
        Ok(Self { graph })
    }

    /// Adds a node to the graph
    pub fn add_node(&mut self, weight: N) -> NodeIndex {
        self.graph.add_node(weight)
    }

    /// Adds an edge to the graph
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, weight: E) -> Result<(), String> {
        self.graph.add_edge(from, to, weight);

        // Verify the graph is still acyclic after adding the edge
        if toposort(&self.graph, None).is_err() {
            // Remove the edge we just added
            if let Some(edge) = self.graph.find_edge(from, to) {
                self.graph.remove_edge(edge);
            }
            return Err("Adding this edge would create a cycle".to_string());
        }

        Ok(())
    }

    /// Returns a reference to the underlying graph
    pub fn graph(&self) -> &DiGraph<N, E> {
        &self.graph
    }

    /// Returns the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Partitions the DAG into multiple subgraphs based on maximum node count.
    ///
    /// This uses a level-based partitioning strategy that respects topological ordering:
    /// - Nodes are grouped by their topological level (distance from root nodes)
    /// - Partitions are filled up to max_nodes_per_partition
    /// - Dependencies are always in earlier or same partitions
    pub fn partition(
        &self,
        max_nodes_per_partition: usize,
    ) -> Result<Vec<Partition<N, E>>, String> {
        if max_nodes_per_partition == 0 {
            return Err("max_nodes_per_partition must be greater than 0".to_string());
        }

        if self.graph.node_count() == 0 {
            return Ok(vec![]);
        }

        // Get topological ordering
        let topo_order = toposort(&self.graph, None).map_err(|_| "Graph contains cycles")?;

        // Calculate levels (distance from nodes with no dependencies)
        let levels = self.calculate_levels(&topo_order);

        // Group nodes by level
        let mut levels_map: FxHashMap<usize, Vec<NodeIndex>> = FxHashMap::default();
        for (node, level) in &levels {
            levels_map.entry(*level).or_default().push(*node);
        }

        // Sort levels
        let mut sorted_levels: Vec<_> = levels_map.keys().copied().collect();
        sorted_levels.sort_unstable();

        // Create partitions by filling them level by level
        let mut partitions = Vec::new();
        let mut current_partition_nodes = Vec::new();

        for level in sorted_levels {
            let nodes_at_level = &levels_map[&level];

            for &node_idx in nodes_at_level {
                if current_partition_nodes.len() >= max_nodes_per_partition {
                    // Finalize current partition
                    partitions.push(current_partition_nodes.clone());
                    current_partition_nodes.clear();
                }
                current_partition_nodes.push(node_idx);
            }
        }

        // Add remaining nodes as final partition
        if !current_partition_nodes.is_empty() {
            partitions.push(current_partition_nodes);
        }

        // Build partition objects with subgraphs
        let result = partitions
            .into_iter()
            .enumerate()
            .map(|(idx, node_indices)| self.build_partition(idx, node_indices))
            .collect();

        Ok(result)
    }

    /// Calculates the level (distance from root) for each node
    fn calculate_levels(&self, topo_order: &[NodeIndex]) -> FxHashMap<NodeIndex, usize> {
        let mut levels: FxHashMap<NodeIndex, usize> = FxHashMap::default();

        for &node in topo_order {
            // Get the maximum level of all incoming neighbors
            let incoming_levels = self
                .graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .filter_map(|pred| levels.get(&pred))
                .max()
                .copied();

            // This node's level is one more than the max of its dependencies
            let level = incoming_levels.map_or(0, |l| l + 1);
            levels.insert(node, level);
        }

        levels
    }

    /// Builds a partition subgraph from a list of node indices
    fn build_partition(&self, index: usize, node_indices: Vec<NodeIndex>) -> Partition<N, E> {
        let node_set: FxHashSet<NodeIndex> = node_indices.iter().copied().collect();

        // Create a mapping from old node indices to new ones
        let mut index_map: FxHashMap<NodeIndex, NodeIndex> = FxHashMap::default();
        let mut subgraph = DiGraph::new();

        // Add nodes to subgraph
        let nodes: Vec<N> = node_indices
            .iter()
            .map(|&old_idx| {
                let weight = self.graph.node_weight(old_idx).unwrap().clone();
                let new_idx = subgraph.add_node(weight.clone());
                index_map.insert(old_idx, new_idx);
                weight
            })
            .collect();

        // Add edges that are fully contained within this partition
        for &old_idx in &node_indices {
            for edge in self.graph.edges(old_idx) {
                let target = edge.target();
                if node_set.contains(&target) {
                    let new_source = index_map[&old_idx];
                    let new_target = index_map[&target];
                    subgraph.add_edge(new_source, new_target, edge.weight().clone());
                }
            }
        }

        Partition {
            nodes,
            graph: subgraph,
            index,
        }
    }

    /// Partitions the DAG using a greedy algorithm that tries to minimize edge cuts
    /// between partitions while respecting the maximum node count.
    pub fn partition_greedy(
        &self,
        max_nodes_per_partition: usize,
    ) -> Result<Vec<Partition<N, E>>, String> {
        if max_nodes_per_partition == 0 {
            return Err("max_nodes_per_partition must be greater than 0".to_string());
        }

        if self.graph.node_count() == 0 {
            return Ok(vec![]);
        }

        // Get topological ordering
        let topo_order = toposort(&self.graph, None).map_err(|_| "Graph contains cycles")?;

        let mut partitions: Vec<Vec<NodeIndex>> = Vec::new();
        let mut current_partition: Vec<NodeIndex> = Vec::new();
        let mut assigned: FxHashSet<NodeIndex> = FxHashSet::default();

        // Process nodes in topological order
        for &node in &topo_order {
            if assigned.contains(&node) {
                continue;
            }

            // Check if all dependencies of this node are already assigned
            let deps_satisfied = self
                .graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .all(|dep| assigned.contains(&dep));

            // If current partition is full or dependencies aren't satisfied, start a new partition
            if current_partition.len() >= max_nodes_per_partition || !deps_satisfied {
                if !current_partition.is_empty() {
                    partitions.push(current_partition.clone());
                    current_partition.clear();
                }
            }

            current_partition.push(node);
            assigned.insert(node);
        }

        // Add remaining nodes
        if !current_partition.is_empty() {
            partitions.push(current_partition);
        }

        // Build partition objects
        let result = partitions
            .into_iter()
            .enumerate()
            .map(|(idx, node_indices)| self.build_partition(idx, node_indices))
            .collect();

        Ok(result)
    }
}

impl<N: Clone, E: Clone> Default for PartitionableDAG<N, E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_dag() {
        let dag: PartitionableDAG<i32, ()> = PartitionableDAG::new();
        let partitions = dag.partition(5).unwrap();
        assert_eq!(partitions.len(), 0);
    }

    #[test]
    fn test_single_node() {
        let mut dag: PartitionableDAG<i32, ()> = PartitionableDAG::new();
        dag.add_node(1);

        let partitions = dag.partition(1).unwrap();
        assert_eq!(partitions.len(), 1);
        assert_eq!(partitions[0].nodes.len(), 1);
    }

    #[test]
    fn test_linear_chain() {
        let mut dag: PartitionableDAG<i32, ()> = PartitionableDAG::new();
        let n1 = dag.add_node(1);
        let n2 = dag.add_node(2);
        let n3 = dag.add_node(3);
        let n4 = dag.add_node(4);

        dag.add_edge(n1, n2, ()).unwrap();
        dag.add_edge(n2, n3, ()).unwrap();
        dag.add_edge(n3, n4, ()).unwrap();

        let partitions = dag.partition(2).unwrap();
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].nodes.len(), 2);
        assert_eq!(partitions[1].nodes.len(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let mut dag: PartitionableDAG<i32, ()> = PartitionableDAG::new();
        let n1 = dag.add_node(1);
        let n2 = dag.add_node(2);
        let n3 = dag.add_node(3);

        dag.add_edge(n1, n2, ()).unwrap();
        dag.add_edge(n2, n3, ()).unwrap();

        // This should fail as it creates a cycle
        let result = dag.add_edge(n3, n1, ());
        assert!(result.is_err());
    }

    #[test]
    fn test_diamond_dag() {
        let mut dag: PartitionableDAG<i32, ()> = PartitionableDAG::new();
        let n1 = dag.add_node(1);
        let n2 = dag.add_node(2);
        let n3 = dag.add_node(3);
        let n4 = dag.add_node(4);

        // Create a diamond: 1 -> 2,3 -> 4
        dag.add_edge(n1, n2, ()).unwrap();
        dag.add_edge(n1, n3, ()).unwrap();
        dag.add_edge(n2, n4, ()).unwrap();
        dag.add_edge(n3, n4, ()).unwrap();

        let partitions = dag.partition(2).unwrap();
        assert!(partitions.len() >= 2);

        // Verify all 4 nodes are present across partitions
        let total_nodes: usize = partitions.iter().map(|p| p.nodes.len()).sum();
        assert_eq!(total_nodes, 4);
    }

    #[test]
    fn test_max_nodes_per_partition() {
        let mut dag: PartitionableDAG<i32, ()> = PartitionableDAG::new();
        for i in 0..10 {
            dag.add_node(i);
        }

        let partitions = dag.partition(3).unwrap();

        // Each partition should have at most 3 nodes
        for partition in &partitions {
            assert!(partition.nodes.len() <= 3);
        }

        // All nodes should be present
        let total_nodes: usize = partitions.iter().map(|p| p.nodes.len()).sum();
        assert_eq!(total_nodes, 10);
    }

    #[test]
    fn test_greedy_partition() {
        let mut dag: PartitionableDAG<i32, ()> = PartitionableDAG::new();
        let n1 = dag.add_node(1);
        let n2 = dag.add_node(2);
        let n3 = dag.add_node(3);
        let n4 = dag.add_node(4);

        dag.add_edge(n1, n3, ()).unwrap();
        dag.add_edge(n2, n4, ()).unwrap();

        let partitions = dag.partition_greedy(2).unwrap();

        // Should create partitions respecting dependencies
        assert!(partitions.len() >= 1);
        let total_nodes: usize = partitions.iter().map(|p| p.nodes.len()).sum();
        assert_eq!(total_nodes, 4);
    }
}
