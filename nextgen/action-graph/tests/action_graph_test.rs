#![allow(clippy::disallowed_names)]

mod utils;

use moon_action_graph::*;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{NodeConfig, RustConfig};
use moon_node_platform::NodePlatform;
use moon_platform::PlatformManager;
use moon_platform_runtime::*;
use moon_project_graph::ProjectGraph;
use moon_rust_platform::RustPlatform;
use moon_task::{Target, Task};
use moon_test_utils2::generate_project_graph;
use proto_core::ProtoEnvironment;
use rustc_hash::FxHashSet;
use starbase_sandbox::{assert_snapshot, create_sandbox};
use std::path::Path;
use std::sync::Arc;
use utils::ActionGraphContainer;

fn create_task(id: &str, project: &str) -> Task {
    Task {
        id: Id::raw(id),
        target: Target::new(project, id).unwrap(),
        ..Task::default()
    }
}

async fn create_project_graph() -> ProjectGraph {
    generate_project_graph("projects").await
}

fn create_node_runtime() -> Runtime {
    Runtime::new(
        PlatformType::Node,
        RuntimeReq::with_version(Version::new(20, 0, 0)),
    )
}

fn create_rust_runtime() -> Runtime {
    Runtime::new(
        PlatformType::Rust,
        RuntimeReq::with_version(Version::new(1, 70, 0)),
    )
}

fn create_platform_manager(root: &Path) -> PlatformManager {
    let mut manager = PlatformManager::default();
    let proto = Arc::new(ProtoEnvironment::new_testing(root));

    manager.register(
        PlatformType::Node,
        Box::new(NodePlatform::new(
            &NodeConfig {
                version: Some(UnresolvedVersionSpec::Version(Version::new(20, 0, 0))),
                ..Default::default()
            },
            &None,
            root,
            proto.clone(),
        )),
    );

    manager.register(
        PlatformType::Rust,
        Box::new(RustPlatform::new(
            &RustConfig {
                version: Some(UnresolvedVersionSpec::Version(Version::new(1, 70, 0))),
                ..Default::default()
            },
            root,
            proto.clone(),
        )),
    );

    manager
}

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

    // #[test]
    // fn errors_on_cycle() {
    //     let mut graph = ProjectGraphType::new();
    //     let a = graph.add_node(create_project("a"));
    //     let b = graph.add_node(create_project("b"));
    //     graph.add_edge(a, b, DependencyScope::Build);
    //     graph.add_edge(b, a, DependencyScope::Build);

    //     let pg = ProjectGraph::new(
    //         graph,
    //         FxHashMap::from_iter([
    //             ("a".into(), ProjectNode::new(0)),
    //             ("b".into(), ProjectNode::new(1)),
    //         ]),
    //         &PathBuf::from("."),
    //     );

    //     let mut builder = ActionGraphBuilder::new(&pg).unwrap();

    //     builder.sync_project(&pg.get("a").unwrap()).unwrap();
    //     builder.sync_project(&pg.get("b").unwrap()).unwrap();

    //     builder.build().unwrap().reset_iterator().unwrap();
    // }

    mod install_deps {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let bar = container.project_graph.get("bar").unwrap();
            builder.install_deps(&bar, None).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: create_node_runtime()
                    },
                    ActionNode::InstallDeps {
                        runtime: create_node_runtime()
                    }
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let bar = container.project_graph.get("bar").unwrap();
            builder.install_deps(&bar, None).unwrap();
            builder.install_deps(&bar, None).unwrap();
            builder.install_deps(&bar, None).unwrap();

            let graph = builder.build().unwrap();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: create_node_runtime()
                    },
                    ActionNode::InstallDeps {
                        runtime: create_node_runtime()
                    }
                ]
            );
        }

        #[tokio::test]
        async fn installs_in_project_when_not_in_depman_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let inside = container.project_graph.get("in").unwrap();
            builder.install_deps(&inside, None).unwrap();

            let outside = container.project_graph.get("out").unwrap();
            builder.install_deps(&outside, None).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: create_node_runtime()
                    },
                    ActionNode::InstallProjectDeps {
                        project: Id::raw("out"),
                        runtime: create_node_runtime()
                    },
                    ActionNode::InstallDeps {
                        runtime: create_node_runtime()
                    },
                ]
            );
        }
    }

    mod run_task {
        use super::*;
        use starbase_sandbox::pretty_assertions::assert_eq;

        #[tokio::test]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.project_graph.get("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.platform = PlatformType::Node;

            builder.run_task(&project, &task, None).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: create_node_runtime()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: create_node_runtime()
                    },
                    ActionNode::InstallDeps {
                        runtime: create_node_runtime()
                    },
                    ActionNode::RunTask {
                        interactive: false,
                        persistent: false,
                        runtime: create_node_runtime(),
                        target: task.target
                    }
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.project_graph.get("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.platform = PlatformType::Node;

            builder.run_task(&project, &task, None).unwrap();
            builder.run_task(&project, &task, None).unwrap();
            builder.run_task(&project, &task, None).unwrap();

            let graph = builder.build().unwrap();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: create_node_runtime()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: create_node_runtime()
                    },
                    ActionNode::InstallDeps {
                        runtime: create_node_runtime()
                    },
                    ActionNode::RunTask {
                        interactive: false,
                        persistent: false,
                        runtime: create_node_runtime(),
                        target: task.target
                    }
                ]
            );
        }

        #[tokio::test]
        async fn doesnt_graph_if_not_affected_by_touched_files() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.project_graph.get("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.platform = PlatformType::Node;

            builder
                // Empty set works fine, just needs to be some
                .run_task(&project, &task, Some(&FxHashSet::default()))
                .unwrap();

            let graph = builder.build().unwrap();

            assert!(topo(graph).is_empty());
        }

        #[tokio::test]
        async fn graphs_if_affected_by_touched_files() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let file = WorkspaceRelativePathBuf::from("bar/file.js");

            let project = container.project_graph.get("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.platform = PlatformType::Node;
            task.input_files.insert(file.clone());

            builder
                .run_task(&project, &task, Some(&FxHashSet::from_iter([file])))
                .unwrap();

            let graph = builder.build().unwrap();

            assert!(!topo(graph).is_empty());
        }

        #[tokio::test]
        async fn task_can_have_a_diff_platform_from_project() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            // node
            let project = container.project_graph.get("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.platform = PlatformType::Rust;

            builder.run_task(&project, &task, None).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: create_node_runtime()
                    },
                    ActionNode::SetupTool {
                        runtime: create_rust_runtime()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: create_node_runtime()
                    },
                    ActionNode::InstallDeps {
                        runtime: create_rust_runtime()
                    },
                    ActionNode::RunTask {
                        interactive: false,
                        persistent: false,
                        runtime: create_rust_runtime(),
                        target: task.target
                    }
                ]
            );
        }

        #[tokio::test]
        async fn sets_interactive() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.project_graph.get("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.options.interactive = true;

            builder.run_task(&project, &task, None).unwrap();

            let graph = builder.build().unwrap();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::RunTask {
                    interactive: true,
                    persistent: false,
                    runtime: Runtime::system(),
                    target: task.target
                }
            );
        }

        #[tokio::test]
        async fn sets_persistent() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.project_graph.get("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.options.persistent = true;

            builder.run_task(&project, &task, None).unwrap();

            let graph = builder.build().unwrap();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::RunTask {
                    interactive: false,
                    persistent: true,
                    runtime: Runtime::system(),
                    target: task.target
                }
            );
        }
    }

    mod setup_tool {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let pg = ProjectGraph::default();
            let mut builder = ActionGraphBuilder::new(&pg).unwrap();
            let system = Runtime::system();
            let node = Runtime::new(
                PlatformType::Node,
                RuntimeReq::with_version(Version::new(1, 2, 3)),
            );

            builder.setup_tool(&system);
            builder.setup_tool(&node);

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool { runtime: node },
                    ActionNode::SetupTool { runtime: system },
                ]
            );
        }

        #[tokio::test]
        async fn graphs_same_platform() {
            let pg = ProjectGraph::default();
            let mut builder = ActionGraphBuilder::new(&pg).unwrap();

            let node1 = Runtime::new(
                PlatformType::Node,
                RuntimeReq::with_version(Version::new(1, 2, 3)),
            );
            let node2 = Runtime::new_override(
                PlatformType::Node,
                RuntimeReq::with_version(Version::new(4, 5, 6)),
            );
            let node3 = Runtime::new(PlatformType::Node, RuntimeReq::Global);

            builder.setup_tool(&node1);
            builder.setup_tool(&node2);
            builder.setup_tool(&node3);

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool { runtime: node3 },
                    ActionNode::SetupTool { runtime: node2 },
                    ActionNode::SetupTool { runtime: node1 },
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let pg = ProjectGraph::default();
            let mut builder = ActionGraphBuilder::new(&pg).unwrap();
            let system = Runtime::system();

            builder.setup_tool(&system);
            builder.setup_tool(&system);

            let graph = builder.build().unwrap();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool { runtime: system },
                ]
            );
        }
    }

    mod sync_project {
        use super::*;

        #[tokio::test]
        async fn graphs_single() {
            let pg = create_project_graph().await;
            let mut builder = ActionGraphBuilder::new(&pg).unwrap();

            let bar = pg.get("bar").unwrap();
            builder.sync_project(&bar).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: Runtime::system()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: Runtime::system()
                    }
                ]
            );
        }

        #[tokio::test]
        async fn graphs_single_with_dep() {
            let pg = create_project_graph().await;
            let mut builder = ActionGraphBuilder::new(&pg).unwrap();

            let foo = pg.get("foo").unwrap();
            builder.sync_project(&foo).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: Runtime::system()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: Runtime::system()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("foo"),
                        runtime: Runtime::system()
                    }
                ]
            );
        }

        #[tokio::test]
        async fn graphs_multiple() {
            let pg = create_project_graph().await;
            let mut builder = ActionGraphBuilder::new(&pg).unwrap();

            let foo = pg.get("foo").unwrap();
            builder.sync_project(&foo).unwrap();

            let bar = pg.get("bar").unwrap();
            builder.sync_project(&bar).unwrap();

            let qux = pg.get("qux").unwrap();
            builder.sync_project(&qux).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: Runtime::system()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("qux"),
                        runtime: Runtime::system()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: Runtime::system()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("foo"),
                        runtime: Runtime::system()
                    },
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let pg = create_project_graph().await;
            let mut builder = ActionGraphBuilder::new(&pg).unwrap();

            let foo = pg.get("foo").unwrap();

            builder.sync_project(&foo).unwrap();
            builder.sync_project(&foo).unwrap();
            builder.sync_project(&foo).unwrap();

            let graph = builder.build().unwrap();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: Runtime::system()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: Runtime::system()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("foo"),
                        runtime: Runtime::system()
                    }
                ]
            );
        }

        #[tokio::test]
        async fn inherits_platform_tool() {
            let pg = create_project_graph().await;
            let pm = create_platform_manager(&pg.workspace_root);
            let mut builder = ActionGraphBuilder::with_platforms(&pm, &pg).unwrap();

            let bar = pg.get("bar").unwrap();
            builder.sync_project(&bar).unwrap();

            let qux = pg.get("qux").unwrap();
            builder.sync_project(&qux).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: create_rust_runtime()
                    },
                    ActionNode::SetupTool {
                        runtime: create_node_runtime()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("qux"),
                        runtime: create_rust_runtime()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: create_node_runtime()
                    }
                ]
            );
        }

        #[tokio::test]
        async fn supports_platform_override() {
            let pg = create_project_graph().await;
            let pm = create_platform_manager(&pg.workspace_root);
            let mut builder = ActionGraphBuilder::with_platforms(&pm, &pg).unwrap();

            let bar = pg.get("bar").unwrap();
            builder.sync_project(&bar).unwrap();

            let baz = pg.get("baz").unwrap();
            builder.sync_project(&baz).unwrap();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::SyncWorkspace,
                    ActionNode::SetupTool {
                        runtime: Runtime::new_override(
                            PlatformType::Node,
                            RuntimeReq::with_version(Version::new(18, 0, 0))
                        )
                    },
                    ActionNode::SetupTool {
                        runtime: create_node_runtime()
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("baz"),
                        runtime: Runtime::new_override(
                            PlatformType::Node,
                            RuntimeReq::with_version(Version::new(18, 0, 0))
                        )
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: create_node_runtime()
                    },
                ]
            );
        }
    }

    mod sync_workspace {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let pg = ProjectGraph::default();

            let mut builder = ActionGraphBuilder::new(&pg).unwrap();
            builder.sync_workspace();

            let graph = builder.build().unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::SyncWorkspace]);
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let pg = ProjectGraph::default();

            let mut builder = ActionGraphBuilder::new(&pg).unwrap();
            builder.sync_workspace();
            builder.sync_workspace();
            builder.sync_workspace();

            let graph = builder.build().unwrap();

            assert_eq!(topo(graph), vec![ActionNode::SyncWorkspace]);
        }
    }
}
