use petgraph::Direction;
use petgraph::algo::{DfsSpace, has_path_connecting, toposort};
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use petgraph::visit::{EdgeRef, Visitable};

/// Traversal state for [`add_acyclic_edge`], which is reused between checks,
/// instead of allocating one for each.
pub type CycleSpace = DfsSpace<NodeIndex, <DiGraph<(), ()> as Visitable>::Map>;

/// Whether adding an edge from `from` to `to` to an acyclic graph can close a
/// cycle. A path back from `to` to `from` needs an edge into `from` and an edge
/// out of `to`, and can't exist when an edge from `from` to `to` already does.
/// A self-loop always closes one.
fn must_check_for_cycle<N, E>(graph: &DiGraph<N, E>, from: NodeIndex, to: NodeIndex) -> bool {
    from == to
        || graph
            .neighbors_directed(from, Direction::Incoming)
            .next()
            .is_some()
            && graph
                .neighbors_directed(to, Direction::Outgoing)
                .next()
                .is_some()
            && graph.find_edge(from, to).is_none()
}

/// Add an edge from `from` to `to` to an acyclic graph, unless it would
/// introduce a cycle (`to` already reaches `from`), in which case the weight
/// is returned. The search is skipped when the edge can't close a cycle.
pub fn add_acyclic_edge<N, E>(
    graph: &mut DiGraph<N, E>,
    from: NodeIndex,
    to: NodeIndex,
    weight: E,
    space: &mut CycleSpace,
) -> Result<EdgeIndex, E> {
    if must_check_for_cycle(graph, from, to) && has_path_connecting(&*graph, to, from, Some(space))
    {
        return Err(weight);
    }

    Ok(graph.add_edge(from, to, weight))
}

/// Add edges to an acyclic graph in bulk, with a single cycle check, instead
/// of a search for each edge. If they would introduce a cycle, none of them
/// are added, and their weights are returned.
pub fn add_acyclic_edges<N, E>(
    graph: &mut DiGraph<N, E>,
    edges: impl IntoIterator<Item = (NodeIndex, NodeIndex, E)>,
) -> Result<(), Vec<E>> {
    let start = graph.edge_count();
    let mut must_check = false;

    for (from, to, weight) in edges {
        must_check = must_check || must_check_for_cycle(graph, from, to);
        graph.add_edge(from, to, weight);
    }

    // Use a topological sort over `is_cyclic_directed`, as the latter
    // recurses per node, which overflows the stack on deep chains
    if must_check && toposort(&*graph, None).is_err() {
        let end = graph.edge_count();

        // Removed from the last, so that the indices of the others don't change
        let mut weights = (start..end)
            .rev()
            .filter_map(|index| graph.remove_edge(EdgeIndex::new(index)))
            .collect::<Vec<_>>();

        weights.reverse();

        return Err(weights);
    }

    Ok(())
}

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

    let mut space = CycleSpace::new(&replay_graph);

    for edge in graph.edge_references() {
        if add_acyclic_edge(
            &mut replay_graph,
            edge.source(),
            edge.target(),
            (),
            &mut space,
        )
        .is_err()
        {
            return Some(edge.id());
        }
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

    #[test]
    fn adds_edges_that_dont_cycle() {
        let mut graph = create_graph(3, &[(0, 1)]);
        let mut space = CycleSpace::default();

        assert!(add_acyclic_edge(&mut graph, 1.into(), 2.into(), (), &mut space).is_ok());
        assert!(add_acyclic_edge(&mut graph, 0.into(), 2.into(), (), &mut space).is_ok());

        // Duplicates are allowed
        assert!(add_acyclic_edge(&mut graph, 0.into(), 1.into(), (), &mut space).is_ok());
        assert_eq!(graph.edge_count(), 4);
    }

    #[test]
    fn rejects_edges_that_cycle() {
        let mut graph = DiGraph::<(), u8>::new();
        let mut space = CycleSpace::default();

        for _ in 0..3 {
            graph.add_node(());
        }

        graph.add_edge(0.into(), 1.into(), 0);
        graph.add_edge(1.into(), 2.into(), 1);

        assert_eq!(
            add_acyclic_edge(&mut graph, 2.into(), 0.into(), 2, &mut space),
            Err(2)
        );
        assert_eq!(
            add_acyclic_edge(&mut graph, 1.into(), 1.into(), 3, &mut space),
            Err(3)
        );
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn adds_edges_in_bulk() {
        let mut graph = create_graph(3, &[]);

        assert!(
            add_acyclic_edges(
                &mut graph,
                [(0, 1), (1, 2), (0, 2)].map(|(from, to)| (from.into(), to.into(), ()))
            )
            .is_ok()
        );
        assert_eq!(graph.edge_count(), 3);
    }

    #[test]
    fn adds_no_edges_in_bulk_when_they_cycle() {
        let mut graph = DiGraph::<(), u8>::new();

        for _ in 0..3 {
            graph.add_node(());
        }

        graph.add_edge(0.into(), 1.into(), 0);

        assert_eq!(
            add_acyclic_edges(
                &mut graph,
                [(1, 2, 1), (2, 0, 2)].map(|(from, to, weight)| (from.into(), to.into(), weight))
            ),
            Err(vec![1, 2])
        );
        assert_eq!(
            graph
                .edge_references()
                .map(|edge| (edge.source().index(), edge.target().index()))
                .collect::<Vec<_>>(),
            [(0, 1)]
        );
    }

    #[test]
    fn adds_deep_chains_in_bulk() {
        let mut graph = create_graph(10_000, &[]);

        assert!(
            add_acyclic_edges(
                &mut graph,
                (1..10_000u32).map(|i| (i.into(), (i - 1).into(), ()))
            )
            .is_ok()
        );
        assert!(add_acyclic_edges(&mut graph, [(0.into(), 9_999.into(), ())]).is_err());
        assert_eq!(graph.edge_count(), 9_999);
    }
}
