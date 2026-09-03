use daggy::{Dag, Walker};
use moon_config::TaskDependencyType;
use petgraph::prelude::*;

// An edge may only be removed, or walked as proof that another edge is
// redundant, when it guarantees that the target action has *completed*.
// `wait` edges only guarantee that the target has *started*, and `cleanup`
// edges run in the opposite direction of the dependency they describe, so
// neither can stand in for a `required` edge.
fn is_reducible<N>(graph: &Dag<N, TaskDependencyType>, edge: EdgeIndex) -> bool {
    graph
        .edge_weight(edge)
        .is_some_and(|weight| weight.is_required_type())
}

enum Step {
    Enter(NodeIndex),
    Exit,
}

/// A weight aware [transitive reduction](https://en.wikipedia.org/wiki/Transitive_reduction).
///
/// Walks the graph depth-first from each root, and removes every edge that is
/// shadowed by a longer path between the same 2 nodes. Mirrors daggy's
/// `Dag::transitive_reduce`, with 2 additional rules:
///
/// - Only edges that guarantee completion (`required`, and the internal
///   `optional`) may be *removed*.
/// - Only such edges may be *walked* when determining whether another edge is
///   redundant. A path that runs through a `wait` or `cleanup` edge is not
///   proof that the shadowed edge is unnecessary — a `wait` edge only blocks
///   until the dependency has started, so the shadowed edge remains the only
///   thing guaranteeing that it has completed.
///
/// For a graph that only contains required-like edges, this produces the exact
/// same result as daggy's implementation.
pub fn transitive_reduce<N>(graph: &mut Dag<N, TaskDependencyType>, roots: Vec<NodeIndex>) {
    let mut ancestors: Vec<NodeIndex> = vec![];

    for root in roots {
        // Iterative depth-first walk (instead of daggy's recursive one), as
        // action graphs can be very deep. Each `Exit` marker pops the node
        // that its matching `Enter` pushed onto the ancestors path.
        let mut stack = vec![Step::Enter(root)];

        while let Some(step) = stack.pop() {
            let node_index = match step {
                Step::Exit => {
                    ancestors.pop();
                    continue;
                }
                Step::Enter(index) => index,
            };

            // Any edge that points from one of our ancestors to one of our
            // children is redundant, as that ancestor already reaches the
            // child through us
            let mut redundant_edges = vec![];
            let mut children = graph.children(node_index);

            while let Some((child_edge, child_index)) = children.walk_next(&*graph) {
                if !is_reducible(graph, child_edge) {
                    continue;
                }

                let mut parents = graph.parents(child_index);

                while let Some((parent_edge, parent_index)) = parents.walk_next(&*graph) {
                    if ancestors.contains(&parent_index) && is_reducible(graph, parent_edge) {
                        redundant_edges.push(parent_edge);
                    }
                }
            }

            // Removing an edge swaps the graph's last edge into the removed
            // slot, which invalidates that index, so remove in descending
            // order to avoid removing the wrong edges
            redundant_edges.sort_unstable();
            redundant_edges.dedup();

            for edge in redundant_edges.into_iter().rev() {
                graph.remove_edge(edge);
            }

            ancestors.push(node_index);
            stack.push(Step::Exit);

            let mut children = graph.children(node_index);

            while let Some((child_edge, child_index)) = children.walk_next(&*graph) {
                if is_reducible(graph, child_edge) {
                    stack.push(Step::Enter(child_index));
                }
            }
        }

        ancestors.clear();
    }
}
