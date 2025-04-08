use moon_action::*;
use moon_action_graph::{ActionGraph, action_graph_builder2::*};
use moon_app_context::AppContext;
use moon_common::Id;
use moon_config::PipelineActionSwitch;
use moon_test_utils2::{AppContextMocker, WorkspaceGraph, generate_workspace_graph};
use starbase_sandbox::{assert_snapshot, create_empty_sandbox};
use std::sync::Arc;

fn mock_app_context() -> Arc<AppContext> {
    Arc::new(AppContextMocker::new(create_empty_sandbox().path()).mock())
}

async fn mock_workspace_graph() -> Arc<WorkspaceGraph> {
    Arc::new(generate_workspace_graph("projects").await)
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

    mod sync_project {
        use super::*;

        #[tokio::test]
        async fn graphs_single() {
            let wg = mock_workspace_graph().await;
            let mut builder =
                ActionGraphBuilder::new(mock_app_context(), wg.clone(), Default::default())
                    .unwrap();

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).await.unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    })
                ]
            );
        }

        #[tokio::test]
        async fn graphs_multiple() {
            let wg = mock_workspace_graph().await;
            let mut builder =
                ActionGraphBuilder::new(mock_app_context(), wg.clone(), Default::default())
                    .unwrap();

            let foo = wg.get_project("foo").unwrap();
            builder.sync_project(&foo).await.unwrap();

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).await.unwrap();

            let qux = wg.get_project("qux").unwrap();
            builder.sync_project(&qux).await.unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("qux"),
                    }),
                ]
            );
        }

        #[tokio::test]
        async fn graphs_without_deps() {
            let wg = mock_workspace_graph().await;
            let mut builder = ActionGraphBuilder::new(
                mock_app_context(),
                wg.clone(),
                ActionGraphBuilderOptions {
                    sync_project_dependencies: false,
                    ..Default::default()
                },
            )
            .unwrap();

            let foo = wg.get_project("foo").unwrap();
            builder.sync_project(&foo).await.unwrap();

            let qux = wg.get_project("qux").unwrap();
            builder.sync_project(&qux).await.unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("qux"),
                    }),
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let wg = mock_workspace_graph().await;
            let mut builder =
                ActionGraphBuilder::new(mock_app_context(), wg.clone(), Default::default())
                    .unwrap();

            let foo = wg.get_project("foo").unwrap();

            builder.sync_project(&foo).await.unwrap();
            builder.sync_project(&foo).await.unwrap();
            builder.sync_project(&foo).await.unwrap();

            let graph = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                    })
                ]
            );
        }

        #[tokio::test]
        async fn doesnt_add_if_disabled() {
            let wg = mock_workspace_graph().await;
            let mut builder = ActionGraphBuilder::new(
                mock_app_context(),
                wg.clone(),
                ActionGraphBuilderOptions {
                    sync_projects: false.into(),
                    ..Default::default()
                },
            )
            .unwrap();

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).await.unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test]
        async fn doesnt_add_if_not_listed() {
            let wg = mock_workspace_graph().await;
            let mut builder = ActionGraphBuilder::new(
                mock_app_context(),
                wg.clone(),
                ActionGraphBuilderOptions {
                    sync_projects: PipelineActionSwitch::Only(vec![Id::raw("foo")]),
                    ..Default::default()
                },
            )
            .unwrap();

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).await.unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test]
        async fn adds_if_listed() {
            let wg = mock_workspace_graph().await;
            let mut builder = ActionGraphBuilder::new(
                mock_app_context(),
                wg.clone(),
                ActionGraphBuilderOptions {
                    sync_projects: PipelineActionSwitch::Only(vec![Id::raw("bar")]),
                    ..Default::default()
                },
            )
            .unwrap();

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).await.unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    })
                ]
            );
        }
    }

    mod sync_workspace {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let mut builder =
                ActionGraphBuilder::new(mock_app_context(), Default::default(), Default::default())
                    .unwrap();

            builder.sync_workspace().await;

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let mut builder =
                ActionGraphBuilder::new(mock_app_context(), Default::default(), Default::default())
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
                Default::default(),
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
