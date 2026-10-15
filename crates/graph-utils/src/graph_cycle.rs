use petgraph::Direction;
use petgraph::algo::{DfsSpace, has_path_connecting, toposort};
use petgraph::graph::{DiGraph, EdgeIndex};
use petgraph::visit::EdgeRef;

/// Return the first edge (in insertion order) that closes a cycle, if any.
///
/// The graph is checked for cycles once, and only when one exists are the
/// edges replayed in insertion order into a scratch graph, to find the edge
/// that closes it. This is the same edge that checking for a path on every
/// insertion would have rejected, without the cost of a traversal per edge.
pub fn find_cycle<N, E>(graph: &DiGraph<N, E>) -> Option<EdgeIndex> {
    // Use a topological sort over `is_cyclic_directed`, as the latter
    // recurses per node, which overflows the stack on deep chains
    if toposort(graph, None).is_ok() {
        return None;
    }

    let mut replay_graph = DiGraph::<(), ()>::with_capacity(graph.node_count(), graph.edge_count());

    for _ in graph.node_indices() {
        replay_graph.add_node(());
    }

    let mut space = DfsSpace::new(&replay_graph);

    for edge in graph.edge_references() {
        let (source, target) = (edge.source(), edge.target());

        // A path back from the target to the source can only exist when the
        // source has dependents and the target has dependencies (or the edge
        // is a self-loop), so skip the traversal otherwise
        let must_check = source == target
            || replay_graph
                .neighbors_directed(source, Direction::Incoming)
                .next()
                .is_some()
                && replay_graph
                    .neighbors_directed(target, Direction::Outgoing)
                    .next()
                    .is_some();

        if must_check && has_path_connecting(&replay_graph, target, source, Some(&mut space)) {
            return Some(edge.id());
        }

        replay_graph.add_edge(source, target, ());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_graph(count: usize, edges: &[(u32, u32)]) -> DiGraph<(), ()> {
        let mut graph = DiGraph::new();

        for _ in 0..count {
            graph.add_node(());
        }

        for (source, target) in edges {
            graph.add_edge((*source).into(), (*target).into(), ());
        }

        graph
    }

    #[test]
    fn returns_none_when_acyclic() {
        assert_eq!(find_cycle(&create_graph(0, &[])), None);
        assert_eq!(
            find_cycle(&create_graph(3, &[(0, 1), (1, 2), (0, 2)])),
            None
        );
    }

    #[test]
    fn allows_duplicate_edges() {
        assert_eq!(find_cycle(&create_graph(2, &[(0, 1), (0, 1)])), None);
    }

    #[test]
    fn returns_self_loop() {
        assert_eq!(
            find_cycle(&create_graph(2, &[(0, 1), (1, 1)])),
            Some(EdgeIndex::new(1))
        );
    }

    #[test]
    fn returns_edge_that_closes_two_node_cycle() {
        assert_eq!(
            find_cycle(&create_graph(2, &[(0, 1), (1, 0)])),
            Some(EdgeIndex::new(1))
        );
    }

    #[test]
    fn returns_last_inserted_edge_of_cycle() {
        // 2 -> 0 closes the cycle, even though it's not the "back" edge
        // in node order, and edges after it are ignored
        assert_eq!(
            find_cycle(&create_graph(4, &[(1, 2), (0, 1), (2, 0), (3, 0), (0, 3)])),
            Some(EdgeIndex::new(2))
        );
    }

    #[test]
    fn returns_first_of_multiple_cycles() {
        assert_eq!(
            find_cycle(&create_graph(4, &[(2, 3), (0, 1), (1, 0), (3, 2)])),
            Some(EdgeIndex::new(2))
        );
    }

    #[test]
    fn handles_deep_chains() {
        let edges = (1..10_000).map(|i| (i, i - 1)).collect::<Vec<_>>();

        assert_eq!(find_cycle(&create_graph(10_000, &edges)), None);

        let mut edges = edges;
        edges.push((0, 9_999));

        assert_eq!(
            find_cycle(&create_graph(10_000, &edges)),
            Some(EdgeIndex::new(9_999))
        );
    }
}
