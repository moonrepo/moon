mod utils;

use moon_action::*;
use moon_action_graph::{ActionGraph, action_graph_builder2::*};
use moon_common::Id;
use moon_config::PipelineActionSwitch;
use starbase_sandbox::{assert_snapshot, create_sandbox};
use utils::ActionGraphContainer2;

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
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

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
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

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
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        sync_project_dependencies: false,
                        ..Default::default()
                    },
                )
                .await;

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
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

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
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        sync_projects: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).await.unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        sync_projects: PipelineActionSwitch::Only(vec![Id::raw("foo")]),
                        ..Default::default()
                    },
                )
                .await;

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).await.unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        sync_projects: PipelineActionSwitch::Only(vec![Id::raw("bar")]),
                        ..Default::default()
                    },
                )
                .await;

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
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.sync_workspace().await;

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.sync_workspace().await;
            builder.sync_workspace().await;
            builder.sync_workspace().await;

            let graph = builder.build();

            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let mut builder = container
                .create_builder_with_options(
                    container.create_workspace_graph().await,
                    ActionGraphBuilderOptions {
                        sync_workspace: false,
                        ..Default::default()
                    },
                )
                .await;

            builder.sync_workspace().await;

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }
}
