use crate::depth::Depth;
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::fmt::Display;
use std::hash::Hash;

pub trait GraphData<N, E, K> {
    fn get_graph(&self) -> &DiGraph<NodeIndex, E>;
    fn get_nodes(&self) -> FxHashMap<NodeIndex, &N>;
    fn get_node_by_index(&self, index: &NodeIndex) -> &N;
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
            .filter_map(|index| {
                let dep_node = graph_impl.get_node_by_index(index);

                if dep_keys.contains(&graph_impl.get_node_key(dep_node)) {
                    Some(dep_node)
                } else {
                    None
                }
            })
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
    deps.sort_by_key(|a| a.1);

    // Then map and only return the keys
    deps.into_iter().map(|dep| dep.0).collect()
}

fn keys_within<N, E, K: Hash + Eq, G: GraphConnections<N, E, K> + ?Sized>(
    graph_impl: &G,
    nodes: &[&N],
    direction: Direction,
    depth: Depth,
) -> Vec<K> {
    let mut keys = vec![];

    graph_impl.traverse_within(nodes, direction, depth, |node, _, _| {
        keys.push(graph_impl.get_node_key(node));
    });

    keys
}

pub trait GraphConnections<N, E, K: Hash + Eq>: GraphData<N, E, K> {
    fn get_node_index(&self, node: &N) -> NodeIndex;

    /// Return a list of direct node keys that the provided node depends on.
    fn dependencies_of(&self, node: &N) -> Vec<K> {
        let graph = self.get_graph();

        graph
            .neighbors_directed(self.get_node_index(node), Direction::Outgoing)
            .map(|idx| self.get_node_key(self.get_node_by_index(&idx)))
            .collect()
    }

    /// Return a list of all node keys that the provided node depends on.
    fn deep_dependencies_of(&self, node: &N) -> Vec<K> {
        traverse_deep(self, node, false)
    }

    /// Return a list of node keys that any of the provided nodes depend on,
    /// within the provided depth. The provided nodes are excluded, and keys
    /// are returned in breadth-first order.
    fn dependencies_within(&self, nodes: &[&N], depth: Depth) -> Vec<K> {
        keys_within(self, nodes, Direction::Outgoing, depth)
    }

    /// Return a list of direct node keys that require the provided node.
    fn dependents_of(&self, node: &N) -> Vec<K> {
        let graph = self.get_graph();

        graph
            .neighbors_directed(self.get_node_index(node), Direction::Incoming)
            .map(|idx| self.get_node_key(self.get_node_by_index(&idx)))
            .collect()
    }

    /// Return a list of all node keys that require the provided node.
    fn deep_dependents_of(&self, node: &N) -> Vec<K> {
        traverse_deep(self, node, true)
    }

    /// Return a list of node keys that require any of the provided nodes,
    /// within the provided depth. The provided nodes are excluded, and keys
    /// are returned in breadth-first order.
    fn dependents_within(&self, nodes: &[&N], depth: Depth) -> Vec<K> {
        keys_within(self, nodes, Direction::Incoming, depth)
    }

    /// Walk the graph breadth-first in the provided direction, starting from
    /// all of the provided nodes at once (multi-source), and call `visit` for
    /// every node within the provided depth. `Outgoing` walks dependencies,
    /// while `Incoming` walks dependents.
    ///
    /// Every node is visited at most once, at its minimum distance from any of
    /// the provided nodes, which are never visited themselves. The callback
    /// receives the reached node, the neighbour it was reached through (which
    /// is one hop closer to the provided nodes), and its distance (1 = direct).
    /// Ties are broken by node index, so the order is deterministic.
    fn traverse_within(
        &self,
        nodes: &[&N],
        direction: Direction,
        depth: Depth,
        visit: impl FnMut(&N, &N, u32),
    ) {
        self.traverse_within_filtered(nodes, direction, depth, |_| true, visit);
    }

    /// Like [`GraphConnections::traverse_within`], but only follows the edges
    /// whose weight passes the provided filter. A node that is only connected
    /// through filtered out edges is never reached.
    fn traverse_within_filtered(
        &self,
        nodes: &[&N],
        direction: Direction,
        depth: Depth,
        edge_filter: impl Fn(&E) -> bool,
        mut visit: impl FnMut(&N, &N, u32),
    ) {
        if depth.is_none() || nodes.is_empty() {
            return;
        }

        let graph = self.get_graph();
        let mut seeds = nodes
            .iter()
            .map(|node| self.get_node_index(node))
            .collect::<Vec<_>>();

        seeds.sort();
        seeds.dedup();

        let mut visited = FxHashSet::from_iter(seeds.iter().copied());
        let mut queue = VecDeque::from_iter(seeds.into_iter().map(|index| (index, 0)));
        let mut neighbors = vec![];

        while let Some((index, distance)) = queue.pop_front() {
            let next_distance = distance + 1;

            if !depth.includes(next_distance) {
                continue;
            }

            neighbors.clear();
            neighbors.extend(
                graph
                    .edges_directed(index, direction)
                    .filter(|edge| edge_filter(edge.weight()))
                    .map(|edge| match direction {
                        Direction::Outgoing => edge.target(),
                        Direction::Incoming => edge.source(),
                    }),
            );
            neighbors.sort();
            neighbors.dedup();

            for neighbor in &neighbors {
                if visited.insert(*neighbor) {
                    visit(
                        self.get_node_by_index(neighbor),
                        self.get_node_by_index(&index),
                        next_distance,
                    );

                    queue.push_back((*neighbor, next_distance));
                }
            }
        }
    }

    /// Return a list of keys for all nodes currently within the graph.
    fn get_node_keys(&self) -> Vec<K> {
        self.get_graph()
            .raw_nodes()
            .iter()
            .map(|ni| self.get_node_key(self.get_node_by_index(&ni.weight)))
            .collect()
    }
}

pub trait GraphConversions<N: Clone + Display, E: Clone + Display, K: Hash + Eq>:
    GraphConnections<N, E, K>
{
    /// Return the graph with display labels.
    fn to_labeled_graph(&self) -> DiGraph<String, String> {
        self.get_graph().map(
            |ni, _| self.get_node_by_index(&ni).to_string(),
            |_, edge| edge.to_string(),
        )
    }

    /// Return the graph focused for the provided node, and only include
    /// dependents or dependencies.
    fn to_focused_graph(&self, focus_node: &N, with_dependents: bool) -> DiGraph<NodeIndex, E> {
        let upstream = FxHashSet::from_iter(self.deep_dependencies_of(focus_node));
        let downstream = FxHashSet::from_iter(if with_dependents {
            self.deep_dependents_of(focus_node)
        } else {
            vec![]
        });
        let focus_key = self.get_node_key(focus_node);

        self.get_graph().filter_map(
            |_, node| {
                let node_key = self.get_node_key(self.get_node_by_index(node));

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Edges point from a dependent to its dependency.
    struct TestGraph {
        graph: DiGraph<NodeIndex, ()>,
        names: Vec<String>,
    }

    impl TestGraph {
        fn new(names: &[&str], edges: &[(&str, &str)]) -> Self {
            let mut graph = DiGraph::new();

            for i in 0..names.len() {
                graph.add_node(NodeIndex::new(i));
            }

            let index = |name: &str| NodeIndex::new(names.iter().position(|n| *n == name).unwrap());

            for (from, to) in edges {
                graph.add_edge(index(from), index(to), ());
            }

            Self {
                graph,
                names: names.iter().map(|name| name.to_string()).collect(),
            }
        }

        fn node(&self, name: &str) -> &String {
            self.names.iter().find(|n| *n == name).unwrap()
        }

        fn nodes(&self, names: &[&str]) -> Vec<&String> {
            names.iter().map(|name| self.node(name)).collect()
        }
    }

    impl GraphData<String, (), String> for TestGraph {
        fn get_graph(&self) -> &DiGraph<NodeIndex, ()> {
            &self.graph
        }

        fn get_nodes(&self) -> FxHashMap<NodeIndex, &String> {
            self.names
                .iter()
                .enumerate()
                .map(|(i, name)| (NodeIndex::new(i), name))
                .collect()
        }

        fn get_node_by_index(&self, index: &NodeIndex) -> &String {
            &self.names[index.index()]
        }

        fn get_node_key(&self, node: &String) -> String {
            node.clone()
        }
    }

    impl GraphConnections<String, (), String> for TestGraph {
        fn get_node_index(&self, node: &String) -> NodeIndex {
            NodeIndex::new(self.names.iter().position(|n| n == node).unwrap())
        }
    }

    // base <- mid1 <- mid2 <- top <- above
    //                 other <-/
    fn create_graph() -> TestGraph {
        TestGraph::new(
            &["base", "other", "mid1", "mid2", "top", "above", "lone"],
            &[
                ("mid1", "base"),
                ("mid2", "mid1"),
                ("top", "mid2"),
                ("top", "other"),
                ("above", "top"),
            ],
        )
    }

    fn visit_all(
        graph: &TestGraph,
        nodes: &[&str],
        direction: Direction,
        depth: Depth,
    ) -> Vec<(String, String, u32)> {
        let mut visits = vec![];

        graph.traverse_within(
            &graph.nodes(nodes),
            direction,
            depth,
            |node, via, distance| {
                visits.push((node.clone(), via.clone(), distance));
            },
        );

        visits
    }

    #[test]
    fn none_returns_empty() {
        let graph = create_graph();

        assert!(
            graph
                .dependents_within(&graph.nodes(&["base"]), Depth::NONE)
                .is_empty()
        );
        assert!(
            graph
                .dependencies_within(&graph.nodes(&["above"]), Depth::NONE)
                .is_empty()
        );
    }

    #[test]
    fn empty_seeds_returns_empty() {
        let graph = create_graph();

        assert!(graph.dependents_within(&[], Depth::Deep).is_empty());
    }

    #[test]
    fn direct_dependents() {
        let graph = create_graph();

        assert_eq!(
            graph.dependents_within(&graph.nodes(&["base"]), Depth::DIRECT),
            ["mid1"]
        );
    }

    #[test]
    fn limited_dependents() {
        let graph = create_graph();

        assert_eq!(
            graph.dependents_within(&graph.nodes(&["base"]), Depth::Limit(2)),
            ["mid1", "mid2"]
        );
        assert_eq!(
            graph.dependents_within(&graph.nodes(&["base"]), Depth::Limit(3)),
            ["mid1", "mid2", "top"]
        );
    }

    #[test]
    fn deep_dependents() {
        let graph = create_graph();

        assert_eq!(
            graph.dependents_within(&graph.nodes(&["base"]), Depth::Deep),
            ["mid1", "mid2", "top", "above"]
        );
        assert!(
            graph
                .dependents_within(&graph.nodes(&["lone"]), Depth::Deep)
                .is_empty()
        );
    }

    #[test]
    fn dependencies_with_ties_by_index() {
        let graph = create_graph();

        // `top` depends on `mid2` and `other`, which are both 2 hops from
        // `above`, and `other` has the lower node index
        assert_eq!(
            graph.dependencies_within(&graph.nodes(&["above"]), Depth::Limit(2)),
            ["top", "other", "mid2"]
        );
        assert_eq!(
            graph.dependencies_within(&graph.nodes(&["above"]), Depth::Deep),
            ["top", "other", "mid2", "mid1", "base"]
        );
    }

    #[test]
    fn excludes_seeds() {
        let graph = create_graph();

        assert_eq!(
            graph.dependents_within(&graph.nodes(&["base", "mid1"]), Depth::Deep),
            ["mid2", "top", "above"]
        );
        assert_eq!(
            graph.dependents_within(&graph.nodes(&["mid1", "mid1"]), Depth::DIRECT),
            ["mid2"]
        );
    }

    #[test]
    fn multi_source_visits_at_minimum_distance() {
        let graph = create_graph();

        // `top` is 3 hops from `base`, but 1 hop from `other`
        assert_eq!(
            visit_all(&graph, &["base", "other"], Direction::Incoming, Depth::Deep),
            [
                ("mid1".into(), "base".into(), 1),
                ("top".into(), "other".into(), 1),
                ("mid2".into(), "mid1".into(), 2),
                ("above".into(), "top".into(), 2),
            ]
        );

        // Seed order doesn't matter
        assert_eq!(
            visit_all(&graph, &["other", "base"], Direction::Incoming, Depth::Deep),
            visit_all(&graph, &["base", "other"], Direction::Incoming, Depth::Deep),
        );
    }

    #[test]
    fn multi_source_limit_uses_minimum_distance() {
        let graph = create_graph();

        // `above` is out of range from `base`, but within range from `other`
        assert_eq!(
            graph.dependents_within(&graph.nodes(&["base"]), Depth::Limit(2)),
            ["mid1", "mid2"]
        );
        assert_eq!(
            graph.dependents_within(&graph.nodes(&["base", "other"]), Depth::Limit(2)),
            ["mid1", "top", "mid2", "above"]
        );
    }

    #[test]
    fn filtered_edges_are_not_followed() {
        let graph = create_graph();
        let mut visits = vec![];

        graph.traverse_within_filtered(
            &graph.nodes(&["base"]),
            Direction::Incoming,
            Depth::Deep,
            |_| false,
            |node, _, _| visits.push(node.clone()),
        );

        assert!(visits.is_empty());

        graph.traverse_within_filtered(
            &graph.nodes(&["base"]),
            Direction::Incoming,
            Depth::Deep,
            |_| true,
            |node, _, _| visits.push(node.clone()),
        );

        assert_eq!(
            visits,
            graph.dependents_within(&graph.nodes(&["base"]), Depth::Deep)
        );
    }

    #[test]
    fn cycles_terminate() {
        let graph = TestGraph::new(&["x", "y", "z"], &[("x", "y"), ("y", "z"), ("z", "x")]);

        assert_eq!(
            graph.dependents_within(&graph.nodes(&["x"]), Depth::Deep),
            ["z", "y"]
        );
        assert_eq!(
            graph.dependencies_within(&graph.nodes(&["x"]), Depth::Deep),
            ["y", "z"]
        );
        assert_eq!(
            graph.dependencies_within(&graph.nodes(&["x"]), Depth::Limit(100)),
            ["y", "z"]
        );
    }
}
