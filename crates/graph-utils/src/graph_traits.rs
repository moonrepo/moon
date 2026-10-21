use crate::depth::Depth;
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::fmt::Display;
use std::hash::Hash;

pub trait GraphData<N, E, K> {
    /// The weight of each node in the graph: either the node itself, or the
    /// node's own index, when nodes are stored separately.
    type NodeWeight;

    fn get_graph(&self) -> &DiGraph<Self::NodeWeight, E>;
    fn get_nodes(&self) -> FxHashMap<NodeIndex, &N>;
    fn get_node_by_index(&self, index: &NodeIndex) -> &N;
    fn get_node_key(&self, node: &N) -> K;
}

/// Walk the graph breadth-first in the provided direction, starting from all
/// of the provided seed indices at once (multi-source), and call `visit` for
/// every newly reached node within the provided depth, with the neighbour it
/// was reached through and its distance (1 = direct). Neighbours are walked in
/// node index order, so the order is deterministic. If `visit` returns false,
/// the node stays visited, but isn't walked through.
///
/// When `include_seeds` is false, the seeds are marked as visited up front, so
/// they are never visited themselves. Otherwise they are only visited once
/// they're reached through an edge (e.g. in a cycle).
fn walk_indices<W, E>(
    graph: &DiGraph<W, E>,
    seeds: &[NodeIndex],
    direction: Direction,
    depth: Depth,
    include_seeds: bool,
    edge_filter: impl Fn(&E) -> bool,
    mut visit: impl FnMut(NodeIndex, NodeIndex, u32) -> bool,
) {
    if depth.is_none() || seeds.is_empty() {
        return;
    }

    let mut seeds = seeds.to_vec();

    seeds.sort();
    seeds.dedup();

    let mut visited = FxHashSet::default();

    if !include_seeds {
        visited.extend(seeds.iter().copied());
    }

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
            // A filtered out node stays visited, so that it's only checked once
            if visited.insert(*neighbor) && visit(*neighbor, index, next_distance) {
                queue.push_back((*neighbor, next_distance));
            }
        }
    }
}

fn indices_within<W, E>(
    graph: &DiGraph<W, E>,
    seeds: &[NodeIndex],
    direction: Direction,
    depth: Depth,
    include_seeds: bool,
) -> Vec<NodeIndex> {
    let mut indices = vec![];

    walk_indices(
        graph,
        seeds,
        direction,
        depth,
        include_seeds,
        |_| true,
        |index, _, _| {
            indices.push(index);
            true
        },
    );

    indices
}

fn map_keys<N, E, K: Hash + Eq, G: GraphConnections<N, E, K> + ?Sized>(
    graph_impl: &G,
    indices: Vec<NodeIndex>,
) -> Vec<K> {
    indices
        .into_iter()
        .map(|index| graph_impl.get_node_key(graph_impl.get_node_by_index(&index)))
        .collect()
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

    /// Return a list of all node keys that the provided node depends on,
    /// in breadth-first order. The provided node is included if it's
    /// reachable from itself (through a cycle).
    fn deep_dependencies_of(&self, node: &N) -> Vec<K> {
        map_keys(
            self,
            self.deep_dependency_indices_of(self.get_node_index(node)),
        )
    }

    /// Index-level variant of [`GraphConnections::deep_dependencies_of`].
    fn deep_dependency_indices_of(&self, index: NodeIndex) -> Vec<NodeIndex> {
        indices_within(
            self.get_graph(),
            &[index],
            Direction::Outgoing,
            Depth::Deep,
            true,
        )
    }

    /// Return a list of node keys that any of the provided nodes depend on,
    /// within the provided depth. The provided nodes are excluded, and keys
    /// are returned in breadth-first order.
    fn dependencies_within(&self, nodes: &[&N], depth: Depth) -> Vec<K> {
        map_keys(
            self,
            self.dependency_indices_within(&self.get_node_indices(nodes), depth),
        )
    }

    /// Index-level variant of [`GraphConnections::dependencies_within`].
    fn dependency_indices_within(&self, indices: &[NodeIndex], depth: Depth) -> Vec<NodeIndex> {
        indices_within(self.get_graph(), indices, Direction::Outgoing, depth, false)
    }

    /// Return a list of direct node keys that require the provided node.
    fn dependents_of(&self, node: &N) -> Vec<K> {
        let graph = self.get_graph();

        graph
            .neighbors_directed(self.get_node_index(node), Direction::Incoming)
            .map(|idx| self.get_node_key(self.get_node_by_index(&idx)))
            .collect()
    }

    /// Return a list of all node keys that require the provided node,
    /// in breadth-first order. The provided node is included if it's
    /// reachable from itself (through a cycle).
    fn deep_dependents_of(&self, node: &N) -> Vec<K> {
        map_keys(
            self,
            self.deep_dependent_indices_of(self.get_node_index(node)),
        )
    }

    /// Index-level variant of [`GraphConnections::deep_dependents_of`].
    fn deep_dependent_indices_of(&self, index: NodeIndex) -> Vec<NodeIndex> {
        indices_within(
            self.get_graph(),
            &[index],
            Direction::Incoming,
            Depth::Deep,
            true,
        )
    }

    /// Return a list of node keys that require any of the provided nodes,
    /// within the provided depth. The provided nodes are excluded, and keys
    /// are returned in breadth-first order.
    fn dependents_within(&self, nodes: &[&N], depth: Depth) -> Vec<K> {
        map_keys(
            self,
            self.dependent_indices_within(&self.get_node_indices(nodes), depth),
        )
    }

    /// Index-level variant of [`GraphConnections::dependents_within`].
    fn dependent_indices_within(&self, indices: &[NodeIndex], depth: Depth) -> Vec<NodeIndex> {
        indices_within(self.get_graph(), indices, Direction::Incoming, depth, false)
    }

    /// Return the graph indices for the provided nodes.
    fn get_node_indices(&self, nodes: &[&N]) -> Vec<NodeIndex> {
        nodes.iter().map(|node| self.get_node_index(node)).collect()
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
        visit: impl FnMut(&N, &N, u32),
    ) {
        self.traverse_within_filtered_nodes(nodes, direction, depth, edge_filter, |_| true, visit);
    }

    /// Like [`GraphConnections::traverse_within_filtered`], but also only
    /// reaches the nodes that pass the provided node filter. A node that is
    /// filtered out is never visited, nor walked through, so the nodes beyond
    /// it are only reached through other paths. The provided nodes are never
    /// filtered.
    fn traverse_within_filtered_nodes(
        &self,
        nodes: &[&N],
        direction: Direction,
        depth: Depth,
        edge_filter: impl Fn(&E) -> bool,
        node_filter: impl Fn(&N) -> bool,
        mut visit: impl FnMut(&N, &N, u32),
    ) {
        walk_indices(
            self.get_graph(),
            &self.get_node_indices(nodes),
            direction,
            depth,
            false,
            edge_filter,
            |index, via, distance| {
                let node = self.get_node_by_index(&index);

                if !node_filter(node) {
                    return false;
                }

                visit(node, self.get_node_by_index(&via), distance);

                true
            },
        );
    }

    /// Return a list of keys for all nodes currently within the graph.
    fn get_node_keys(&self) -> Vec<K> {
        self.get_graph()
            .node_indices()
            .map(|index| self.get_node_key(self.get_node_by_index(&index)))
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

    /// Return the graph focused for the provided node, which includes all of
    /// its dependencies, and its dependents within the provided depth. Node
    /// weights are the indices of the nodes in this graph.
    fn to_focused_graph(&self, focus_node: &N, dependents: Depth) -> DiGraph<NodeIndex, E> {
        let graph = self.get_graph();
        let focus_index = self.get_node_index(focus_node);
        let mut included = FxHashSet::default();

        // Self
        included.insert(focus_index);

        // Dependencies
        walk_indices(
            graph,
            &[focus_index],
            Direction::Outgoing,
            Depth::Deep,
            true,
            |_| true,
            |index, _, _| {
                included.insert(index);
                true
            },
        );

        // Dependents
        walk_indices(
            graph,
            &[focus_index],
            Direction::Incoming,
            dependents,
            false,
            |_| true,
            |index, _, _| {
                included.insert(index);
                true
            },
        );

        graph.filter_map(
            |index, _| included.contains(&index).then_some(index),
            |_, edge| Some(edge.to_owned()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestEdge;

    impl Display for TestEdge {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("dep")
        }
    }

    /// Edges point from a dependent to its dependency.
    struct TestGraph {
        graph: DiGraph<NodeIndex, TestEdge>,
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
                graph.add_edge(index(from), index(to), TestEdge);
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

    impl GraphData<String, TestEdge, String> for TestGraph {
        type NodeWeight = NodeIndex;

        fn get_graph(&self) -> &DiGraph<NodeIndex, TestEdge> {
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

    impl GraphConnections<String, TestEdge, String> for TestGraph {
        fn get_node_index(&self, node: &String) -> NodeIndex {
            NodeIndex::new(self.names.iter().position(|n| n == node).unwrap())
        }
    }

    impl GraphConversions<String, TestEdge, String> for TestGraph {}

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
    fn filtered_nodes_are_not_visited_or_walked_through() {
        let graph = create_graph();
        let mut visits = vec![];

        // `mid2` is filtered out, so `top` and `above` (only reachable
        // through it from `base`) are never reached
        graph.traverse_within_filtered_nodes(
            &graph.nodes(&["base"]),
            Direction::Incoming,
            Depth::Deep,
            |_| true,
            |node| node != "mid2",
            |node, via, distance| visits.push((node.clone(), via.clone(), distance)),
        );

        assert_eq!(visits, [("mid1".into(), "base".into(), 1)]);

        // But they're still reached through another path
        visits.clear();

        graph.traverse_within_filtered_nodes(
            &graph.nodes(&["base", "other"]),
            Direction::Incoming,
            Depth::Deep,
            |_| true,
            |node| node != "mid2",
            |node, via, distance| visits.push((node.clone(), via.clone(), distance)),
        );

        assert_eq!(
            visits,
            [
                ("mid1".into(), "base".into(), 1),
                ("top".into(), "other".into(), 1),
                ("above".into(), "top".into(), 2),
            ]
        );

        // Seeds are never filtered
        visits.clear();

        graph.traverse_within_filtered_nodes(
            &graph.nodes(&["base"]),
            Direction::Incoming,
            Depth::DIRECT,
            |_| true,
            |node| node != "base",
            |node, via, distance| visits.push((node.clone(), via.clone(), distance)),
        );

        assert_eq!(visits, [("mid1".into(), "base".into(), 1)]);
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

    // top -> left -> bottom
    //    \-> right -/
    fn create_diamond_graph() -> TestGraph {
        TestGraph::new(
            &["top", "left", "right", "bottom"],
            &[
                ("top", "left"),
                ("top", "right"),
                ("left", "bottom"),
                ("right", "bottom"),
            ],
        )
    }

    #[test]
    fn deep_dependencies_in_breadth_first_order() {
        let graph = create_graph();

        // `top` depends on `mid2` and `other`, which are walked in node index order
        assert_eq!(
            graph.deep_dependencies_of(graph.node("above")),
            ["top", "other", "mid2", "mid1", "base"]
        );
        assert_eq!(
            graph.deep_dependencies_of(graph.node("mid2")),
            ["mid1", "base"]
        );
        assert!(graph.deep_dependencies_of(graph.node("base")).is_empty());
        assert!(graph.deep_dependencies_of(graph.node("lone")).is_empty());
    }

    #[test]
    fn deep_dependents_in_breadth_first_order() {
        let graph = create_graph();

        assert_eq!(
            graph.deep_dependents_of(graph.node("base")),
            ["mid1", "mid2", "top", "above"]
        );
        assert_eq!(
            graph.deep_dependents_of(graph.node("other")),
            ["top", "above"]
        );
        assert!(graph.deep_dependents_of(graph.node("above")).is_empty());
        assert!(graph.deep_dependents_of(graph.node("lone")).is_empty());
    }

    #[test]
    fn deep_includes_start_node_when_reachable_from_itself() {
        let graph = TestGraph::new(&["x", "y", "z"], &[("x", "y"), ("y", "z"), ("z", "x")]);

        assert_eq!(graph.deep_dependencies_of(graph.node("x")), ["y", "z", "x"]);
        assert_eq!(graph.deep_dependents_of(graph.node("x")), ["z", "y", "x"]);
        assert_eq!(graph.deep_dependencies_of(graph.node("z")), ["x", "y", "z"]);

        let graph = TestGraph::new(&["a", "b"], &[("a", "b"), ("b", "a")]);

        assert_eq!(graph.deep_dependencies_of(graph.node("a")), ["b", "a"]);
        assert_eq!(graph.deep_dependents_of(graph.node("a")), ["b", "a"]);

        // But not when it isn't
        let graph = TestGraph::new(&["a", "b", "c"], &[("a", "b"), ("b", "c"), ("c", "b")]);

        assert_eq!(graph.deep_dependencies_of(graph.node("a")), ["b", "c"]);
        assert!(graph.deep_dependents_of(graph.node("a")).is_empty());
    }

    #[test]
    fn deep_visits_diamond_nodes_once() {
        let graph = create_diamond_graph();

        assert_eq!(
            graph.deep_dependencies_of(graph.node("top")),
            ["left", "right", "bottom"]
        );
        assert_eq!(
            graph.deep_dependents_of(graph.node("bottom")),
            ["left", "right", "top"]
        );
    }

    #[test]
    fn index_and_key_apis_agree() {
        let to_keys = |graph: &TestGraph, indices: Vec<NodeIndex>| {
            indices
                .iter()
                .map(|index| graph.get_node_by_index(index).clone())
                .collect::<Vec<_>>()
        };

        for graph in [
            create_graph(),
            create_diamond_graph(),
            TestGraph::new(&["x", "y", "z"], &[("x", "y"), ("y", "z"), ("z", "x")]),
        ] {
            for name in graph.names.clone() {
                let node = graph.node(&name);
                let index = graph.get_node_index(node);

                assert_eq!(
                    to_keys(&graph, graph.deep_dependency_indices_of(index)),
                    graph.deep_dependencies_of(node)
                );
                assert_eq!(
                    to_keys(&graph, graph.deep_dependent_indices_of(index)),
                    graph.deep_dependents_of(node)
                );

                for depth in [Depth::NONE, Depth::DIRECT, Depth::Limit(2), Depth::Deep] {
                    assert_eq!(
                        to_keys(&graph, graph.dependency_indices_within(&[index], depth)),
                        graph.dependencies_within(&[node], depth)
                    );
                    assert_eq!(
                        to_keys(&graph, graph.dependent_indices_within(&[index], depth)),
                        graph.dependents_within(&[node], depth)
                    );
                }
            }
        }
    }

    fn focused_names(graph: &TestGraph, node: &str, dependents: Depth) -> Vec<String> {
        let focused = graph.to_focused_graph(graph.node(node), dependents);
        let mut names = focused
            .node_weights()
            .map(|index| graph.get_node_by_index(index).clone())
            .collect::<Vec<_>>();

        names.sort();
        names
    }

    #[test]
    fn focused_graph_includes_dependents_within_depth() {
        let graph = create_graph();

        // Dependencies are always included
        assert_eq!(focused_names(&graph, "mid1", Depth::NONE), ["base", "mid1"]);
        assert_eq!(
            focused_names(&graph, "mid1", Depth::DIRECT),
            ["base", "mid1", "mid2"]
        );
        assert_eq!(
            focused_names(&graph, "mid1", Depth::Limit(2)),
            ["base", "mid1", "mid2", "top"]
        );
        // `other` is a dependency of a dependent, not of the focused node
        assert_eq!(
            focused_names(&graph, "mid1", Depth::Deep),
            ["above", "base", "mid1", "mid2", "top"]
        );
    }

    #[test]
    fn focused_graph_keeps_original_node_weights() {
        let graph = create_diamond_graph();
        let focused = graph.to_focused_graph(graph.node("left"), Depth::DIRECT);

        // `right` is excluded, so `bottom` shifts down an index, but keeps its weight
        assert_eq!(
            focused.node_weights().copied().collect::<Vec<_>>(),
            [NodeIndex::new(0), NodeIndex::new(1), NodeIndex::new(3)]
        );
        assert_eq!(
            focused
                .raw_edges()
                .iter()
                .map(|edge| (focused[edge.source()], focused[edge.target()]))
                .collect::<Vec<_>>(),
            [
                (NodeIndex::new(0), NodeIndex::new(1)),
                (NodeIndex::new(1), NodeIndex::new(3)),
            ]
        );

        // Cycles include every node
        let graph = TestGraph::new(&["x", "y", "z"], &[("x", "y"), ("y", "z"), ("z", "x")]);

        assert_eq!(focused_names(&graph, "x", Depth::NONE), ["x", "y", "z"]);
    }

    #[test]
    fn focused_graph_keeps_edges_between_included_nodes() {
        let graph = create_graph();

        assert_eq!(
            graph
                .to_focused_graph(graph.node("mid1"), Depth::NONE)
                .edge_count(),
            1
        );
        assert_eq!(
            graph
                .to_focused_graph(graph.node("mid1"), Depth::Deep)
                .edge_count(),
            4
        );
    }
}
