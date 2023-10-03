use moon_action_graph::*;
use moon_project_graph::ProjectGraph;
use starbase_sandbox::assert_snapshot;

fn topo(mut graph: ActionGraph) -> Vec<ActionNode> {
    let mut nodes = vec![];

    graph.reset_iterator().unwrap();

    for node in graph {
        nodes.push(node);
    }

    nodes
}

mod action_graph {
    use super::*;

    mod sync_workspace {
        use super::*;

        #[test]
        fn graphs() {
            let pg = ProjectGraph::default();

            let mut builder = ActionGraphBuilder::new(&pg).unwrap();
            builder.sync_workspace();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::SyncWorkspace]);
        }
    }
}
