use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use std::fmt::Display;

pub trait GraphData<N, E, K> {
    fn get_graph(&self) -> &DiGraph<N, E>;
    fn get_node_index(&self, node: &N) -> NodeIndex;
    fn get_node_key(&self, node: &N) -> K;
}

pub trait GraphConnections<N, E, K>: GraphData<N, E, K> {
    /// Return a list of node keys that the provided node depends on.
    fn dependencies_of(&self, node: &N) -> Vec<K> {
        let graph = self.get_graph();

        graph
            .neighbors_directed(self.get_node_index(node), Direction::Outgoing)
            .map(|idx| self.get_node_key(graph.node_weight(idx).unwrap()))
            .collect()
    }

    /// Return a list of node keys that require the provided node.
    fn dependents_of(&self, node: &N) -> Vec<K> {
        let graph = self.get_graph();

        graph
            .neighbors_directed(self.get_node_index(node), Direction::Incoming)
            .map(|idx| self.get_node_key(graph.node_weight(idx).unwrap()))
            .collect()
    }

    /// Return a list of keys for all nodes currently within the graph.
    fn get_node_keys(&self) -> Vec<K> {
        self.get_graph()
            .raw_nodes()
            .iter()
            .map(|n| self.get_node_key(&n.weight))
            .collect()
    }
}

pub trait GraphConversions<N: Clone + Display, E: Clone + Display, K: PartialEq>:
    GraphConnections<N, E, K>
{
    /// Return the graph with display labels.
    fn to_labelled_graph(&self) -> DiGraph<String, String> {
        self.get_graph()
            .map(|_, node| node.to_string(), |_, edge| edge.to_string())
    }

    /// Return the graph focused for the provided node, and only include direct
    /// dependents or dependencies.
    fn to_focused_graph(&self, focus_node: &N, with_dependents: bool) -> DiGraph<N, E> {
        let upstream = self.dependencies_of(focus_node);
        let downstream = self.dependents_of(focus_node);
        let focus_key = self.get_node_key(focus_node);

        self.get_graph().filter_map(
            |_, node| {
                let node_key = self.get_node_key(node);

                if
                // Self
                node_key == focus_key ||
                    // Dependencies
                    upstream.contains(&node_key) ||
                    // Dependents
                    with_dependents && downstream.contains(&node_key)
                {
                    Some(node.to_owned())
                } else {
                    None
                }
            },
            |_, edge| Some(edge.to_owned()),
        )
    }
}
