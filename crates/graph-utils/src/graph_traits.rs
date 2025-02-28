use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Display;
use std::hash::Hash;

pub trait GraphData<N, E, K> {
    fn get_graph(&self) -> &DiGraph<N, E>;
    fn get_node_index(&self, node: &N) -> NodeIndex;
    fn get_node_key(&self, node: &N) -> K;
}

fn traverse_deep<N, E, K: Hash + Eq, G: GraphConnections<N, E, K> + ?Sized>(
    graph_impl: &G,
    node_to_traverse: &N,
    for_dependents: bool,
) -> Vec<K> {
    let graph = graph_impl.get_graph();
    let mut deps = FxHashMap::default();
    let mut dep_order = 0;
    let mut queue = vec![node_to_traverse];
    let mut visited = FxHashSet::default();

    while let Some(node) = queue.pop() {
        let key = graph_impl.get_node_key(node);

        if visited.contains(&key) {
            continue;
        } else {
            visited.insert(key);
        }

        let dep_keys = if for_dependents {
            graph_impl.dependents_of(node)
        } else {
            graph_impl.dependencies_of(node)
        };

        if dep_keys.is_empty() {
            continue;
        }

        let dep_nodes = graph
            .node_weights()
            .filter(|weight| dep_keys.contains(&graph_impl.get_node_key(weight)))
            .collect::<Vec<_>>();

        queue.extend(dep_nodes);

        for dep_key in dep_keys {
            if deps.contains_key(&dep_key) {
                continue;
            }

            deps.insert(dep_key, dep_order);
            dep_order += 1;
        }
    }

    // Sort keys by insertion order
    let mut deps = deps.into_iter().collect::<Vec<_>>();
    deps.sort_by(|a, d| a.1.cmp(&d.1));

    // Then map and only return the keys
    deps.into_iter().map(|dep| dep.0).collect()
}

pub trait GraphConnections<N, E, K: Hash + Eq>: GraphData<N, E, K> {
    /// Return a list of direct node keys that the provided node depends on.
    fn dependencies_of(&self, node: &N) -> Vec<K> {
        let graph = self.get_graph();

        graph
            .neighbors_directed(self.get_node_index(node), Direction::Outgoing)
            .map(|idx| self.get_node_key(graph.node_weight(idx).unwrap()))
            .collect()
    }

    /// Return a list of all node keys that the provided node depends on.
    fn deep_dependencies_of(&self, node: &N) -> Vec<K> {
        traverse_deep(self, node, false)
    }

    /// Return a list of direct node keys that require the provided node.
    fn dependents_of(&self, node: &N) -> Vec<K> {
        let graph = self.get_graph();

        graph
            .neighbors_directed(self.get_node_index(node), Direction::Incoming)
            .map(|idx| self.get_node_key(graph.node_weight(idx).unwrap()))
            .collect()
    }

    /// Return a list of all node keys that require the provided node.
    fn deep_dependents_of(&self, node: &N) -> Vec<K> {
        traverse_deep(self, node, true)
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

pub trait GraphConversions<N: Clone + Display, E: Clone + Display, K: Hash + Eq>:
    GraphConnections<N, E, K>
{
    /// Return the graph with display labels.
    fn to_labeled_graph(&self) -> DiGraph<String, String> {
        self.get_graph()
            .map(|_, node| node.to_string(), |_, edge| edge.to_string())
    }

    /// Return the graph focused for the provided node, and only include
    /// dependents or dependencies.
    fn to_focused_graph(&self, focus_node: &N, with_dependents: bool) -> DiGraph<N, E> {
        let upstream = FxHashSet::from_iter(self.deep_dependencies_of(focus_node));
        let downstream = FxHashSet::from_iter(if with_dependents {
            self.deep_dependents_of(focus_node)
        } else {
            vec![]
        });
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
                    downstream.contains(&node_key)
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
