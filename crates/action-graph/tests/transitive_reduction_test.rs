use daggy::Dag;
use moon_action_graph::transitive_reduce;
use moon_config::TaskDependencyType;
use petgraph::graph::NodeIndex;

type TestDag = Dag<&'static str, TaskDependencyType>;

fn create_graph(
    nodes: Vec<&'static str>,
    edges: Vec<(usize, usize, TaskDependencyType)>,
) -> (TestDag, Vec<NodeIndex>) {
    let mut graph = TestDag::new();
    let indexes = nodes
        .into_iter()
        .map(|node| graph.add_node(node))
        .collect::<Vec<_>>();

    for (from, to, edge_type) in edges {
        graph
            .add_edge(indexes[from], indexes[to], edge_type)
            .unwrap();
    }

    (graph, indexes)
}

fn map_edges(graph: &TestDag) -> Vec<(&'static str, &'static str, String)> {
    let mut edges = graph
        .graph()
        .edge_indices()
        .map(|edge| {
            let (from, to) = graph.edge_endpoints(edge).unwrap();

            (
                *graph.node_weight(from).unwrap(),
                *graph.node_weight(to).unwrap(),
                graph.edge_weight(edge).unwrap().to_string(),
            )
        })
        .collect::<Vec<_>>();
    edges.sort();
    edges
}

mod transitive_reduction {
    use super::*;

    #[test]
    fn removes_shadowed_required_edges() {
        // a -> b -> c, with a shortcut of a -> c
        let (mut graph, indexes) = create_graph(
            vec!["a", "b", "c"],
            vec![
                (0, 1, TaskDependencyType::Required),
                (1, 2, TaskDependencyType::Required),
                (0, 2, TaskDependencyType::Required),
            ],
        );

        transitive_reduce(&mut graph, vec![indexes[0]]);

        assert_eq!(
            map_edges(&graph),
            vec![("a", "b", "required".into()), ("b", "c", "required".into()),]
        );
    }

    #[test]
    fn removes_shadowed_optional_edges() {
        // The internal optional type is treated like required
        let (mut graph, indexes) = create_graph(
            vec!["a", "b", "c"],
            vec![
                (0, 1, TaskDependencyType::Optional),
                (1, 2, TaskDependencyType::Required),
                (0, 2, TaskDependencyType::Optional),
            ],
        );

        transitive_reduce(&mut graph, vec![indexes[0]]);

        assert_eq!(
            map_edges(&graph),
            vec![("a", "b", "optional".into()), ("b", "c", "required".into()),]
        );
    }

    #[test]
    fn matches_daggy_for_required_only_graphs() {
        // A diamond with every possible shortcut
        let edges = vec![
            (0, 1, TaskDependencyType::Required),
            (0, 2, TaskDependencyType::Required),
            (0, 3, TaskDependencyType::Required),
            (0, 4, TaskDependencyType::Required),
            (1, 3, TaskDependencyType::Required),
            (2, 3, TaskDependencyType::Required),
            (3, 4, TaskDependencyType::Required),
        ];

        let (mut graph, indexes) = create_graph(vec!["a", "b", "c", "d", "e"], edges.clone());
        let (mut daggy_graph, daggy_indexes) = create_graph(vec!["a", "b", "c", "d", "e"], edges);

        transitive_reduce(&mut graph, vec![indexes[0]]);
        daggy_graph.transitive_reduce(vec![daggy_indexes[0]]);

        assert_eq!(map_edges(&graph), map_edges(&daggy_graph));
        assert_eq!(
            map_edges(&graph),
            vec![
                ("a", "b", "required".into()),
                ("a", "c", "required".into()),
                ("b", "d", "required".into()),
                ("c", "d", "required".into()),
                ("d", "e", "required".into()),
            ]
        );
    }

    #[test]
    fn doesnt_remove_an_edge_shadowed_by_a_wait_path() {
        // a -> b (required), b -> c (wait), a -> c (required)
        //
        // `b` can complete while `c` is still running, so the a -> c edge is
        // the only thing that guarantees `c` completes before `a` runs
        let edges = vec![
            (0, 1, TaskDependencyType::Required),
            (1, 2, TaskDependencyType::Wait),
            (0, 2, TaskDependencyType::Required),
        ];

        let (mut graph, indexes) = create_graph(vec!["a", "b", "c"], edges.clone());

        transitive_reduce(&mut graph, vec![indexes[0]]);

        assert_eq!(
            map_edges(&graph),
            vec![
                ("a", "b", "required".into()),
                ("a", "c", "required".into()),
                ("b", "c", "wait".into()),
            ]
        );

        // While daggy would remove it, as it doesn't know about weights
        let (mut daggy_graph, daggy_indexes) = create_graph(vec!["a", "b", "c"], edges);

        daggy_graph.transitive_reduce(vec![daggy_indexes[0]]);

        assert_eq!(
            map_edges(&daggy_graph),
            vec![("a", "b", "required".into()), ("b", "c", "wait".into()),]
        );
    }

    #[test]
    fn doesnt_remove_a_shadowed_wait_edge() {
        // a -> b (required), b -> c (required), a -> c (wait)
        let (mut graph, indexes) = create_graph(
            vec!["a", "b", "c"],
            vec![
                (0, 1, TaskDependencyType::Required),
                (1, 2, TaskDependencyType::Required),
                (0, 2, TaskDependencyType::Wait),
            ],
        );

        transitive_reduce(&mut graph, vec![indexes[0]]);

        assert_eq!(
            map_edges(&graph),
            vec![
                ("a", "b", "required".into()),
                ("a", "c", "wait".into()),
                ("b", "c", "required".into()),
            ]
        );
    }

    #[test]
    fn doesnt_remove_a_shadowed_cleanup_edge() {
        // a -> b (required), b -> c (required), a -> c (cleanup)
        let (mut graph, indexes) = create_graph(
            vec!["a", "b", "c"],
            vec![
                (0, 1, TaskDependencyType::Required),
                (1, 2, TaskDependencyType::Required),
                (0, 2, TaskDependencyType::Cleanup),
            ],
        );

        transitive_reduce(&mut graph, vec![indexes[0]]);

        assert_eq!(
            map_edges(&graph),
            vec![
                ("a", "b", "required".into()),
                ("a", "c", "cleanup".into()),
                ("b", "c", "required".into()),
            ]
        );
    }

    #[test]
    fn doesnt_walk_through_a_cleanup_edge() {
        // a -> b (cleanup), b -> c (required), a -> c (required)
        let (mut graph, indexes) = create_graph(
            vec!["a", "b", "c"],
            vec![
                (0, 1, TaskDependencyType::Cleanup),
                (1, 2, TaskDependencyType::Required),
                (0, 2, TaskDependencyType::Required),
            ],
        );

        transitive_reduce(&mut graph, vec![indexes[0]]);

        assert_eq!(
            map_edges(&graph),
            vec![
                ("a", "b", "cleanup".into()),
                ("a", "c", "required".into()),
                ("b", "c", "required".into()),
            ]
        );
    }

    #[test]
    fn removes_multiple_edges_in_one_pass() {
        // a -> b -> c -> d, with shortcuts from a to c and d
        let (mut graph, indexes) = create_graph(
            vec!["a", "b", "c", "d"],
            vec![
                (0, 1, TaskDependencyType::Required),
                (1, 2, TaskDependencyType::Required),
                (2, 3, TaskDependencyType::Required),
                (0, 2, TaskDependencyType::Required),
                (0, 3, TaskDependencyType::Required),
                (1, 3, TaskDependencyType::Required),
            ],
        );

        transitive_reduce(&mut graph, vec![indexes[0]]);

        assert_eq!(
            map_edges(&graph),
            vec![
                ("a", "b", "required".into()),
                ("b", "c", "required".into()),
                ("c", "d", "required".into()),
            ]
        );
    }

    #[test]
    fn only_reduces_from_the_provided_roots() {
        // Edges point from the dependent to the dependency, so passing a leaf
        // (like the action graph's sync workspace node) walks nothing
        let (mut graph, indexes) = create_graph(
            vec!["a", "b", "c"],
            vec![
                (0, 1, TaskDependencyType::Required),
                (1, 2, TaskDependencyType::Required),
                (0, 2, TaskDependencyType::Required),
            ],
        );

        transitive_reduce(&mut graph, vec![indexes[2]]);

        assert_eq!(map_edges(&graph).len(), 3);
    }
}
