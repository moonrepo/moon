use moon_action::ActionNode;
use moon_action_graph::{ActionGraph, action_graph_builder2::*};
use moon_app_context::AppContext;
use moon_test_utils2::{AppContextMocker, WorkspaceGraph};
use starbase_sandbox::{assert_snapshot, create_empty_sandbox};
use std::sync::Arc;

fn mock_app_context() -> Arc<AppContext> {
    Arc::new(AppContextMocker::new(create_empty_sandbox().path()).mock())
}

fn mock_workspace_graph() -> Arc<WorkspaceGraph> {
    Default::default()
}

fn topo(graph: ActionGraph) -> Vec<ActionNode> {
    let mut nodes = vec![];

    for index in graph.sort_topological().unwrap() {
        nodes.push(graph.get_node_from_index(&index).unwrap().to_owned());
    }

    nodes
}

mod action_graph_builder {
    use super::*;

    mod sync_workspace {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let mut builder = ActionGraphBuilder::new(
                mock_app_context(),
                mock_workspace_graph(),
                Default::default(),
            )
            .unwrap();

            builder.sync_workspace().await;

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let mut builder = ActionGraphBuilder::new(
                mock_app_context(),
                mock_workspace_graph(),
                Default::default(),
            )
            .unwrap();

            builder.sync_workspace().await;
            builder.sync_workspace().await;
            builder.sync_workspace().await;

            let graph = builder.build();

            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test]
        async fn doesnt_add_if_disabled() {
            let mut builder = ActionGraphBuilder::new(
                mock_app_context(),
                mock_workspace_graph(),
                ActionGraphBuilderOptions {
                    sync_workspace: false,
                    ..Default::default()
                },
            )
            .unwrap();

            builder.sync_workspace().await;

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }
}
