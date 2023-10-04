use moon_action_graph::*;
use moon_common::Id;
use moon_config::{
    DependencyConfig, DependencyScope, DependencySource, LanguageType, NodeConfig,
    ProjectToolchainCommonToolConfig, RustConfig,
};
use moon_node_platform::NodePlatform;
use moon_platform::PlatformManager;
use moon_platform_runtime::*;
use moon_project::Project;
use moon_project_graph::{GraphType as ProjectGraphType, ProjectGraph, ProjectNode};
use moon_rust_platform::RustPlatform;
use proto_core::ProtoEnvironment;
use rustc_hash::FxHashMap;
use starbase_sandbox::assert_snapshot;
use std::path::PathBuf;
use std::sync::Arc;

fn create_project(id: &str) -> Project {
    let mut project = Project::default();
    project.id = Id::raw(id);
    project
}

fn create_project_graph() -> ProjectGraph {
    // Create projects
    let mut foo = create_project("foo");
    foo.dependencies.insert(
        Id::raw("bar"),
        DependencyConfig {
            id: Id::raw("bar"),
            scope: DependencyScope::Production,
            source: DependencySource::Explicit,
            via: None,
        },
    );

    let mut bar = create_project("bar");
    bar.language = LanguageType::JavaScript;
    bar.platform = PlatformType::Node;

    let mut baz = create_project("baz");
    baz.language = LanguageType::TypeScript;
    baz.platform = PlatformType::Node;
    baz.config.toolchain.node = Some(ProjectToolchainCommonToolConfig {
        version: Some(UnresolvedVersionSpec::Version(Version::new(18, 0, 0))),
    });

    let mut qux = create_project("qux");
    qux.language = LanguageType::Rust;
    qux.platform = PlatformType::Rust;

    // Map nodes and create graph (in order of expected insertion)
    let mut nodes = FxHashMap::default();
    let mut graph = ProjectGraphType::new();

    let bi = graph.add_node(bar);
    nodes.insert(
        "bar".into(),
        ProjectNode {
            alias: None,
            index: bi,
            source: "bar".into(),
        },
    );

    let fi = graph.add_node(foo);
    nodes.insert(
        "foo".into(),
        ProjectNode {
            alias: None,
            index: fi,
            source: "foo".into(),
        },
    );

    graph.add_edge(fi, bi, DependencyScope::Production);

    let zi = graph.add_node(baz);
    nodes.insert(
        "baz".into(),
        ProjectNode {
            alias: None,
            index: zi,
            source: "baz".into(),
        },
    );

    let qi = graph.add_node(qux);
    nodes.insert(
        "qux".into(),
        ProjectNode {
            alias: None,
            index: qi,
            source: "qux".into(),
        },
    );

    ProjectGraph::new(graph, nodes, &PathBuf::from("."))
}

fn create_platform_manager() -> PlatformManager {
    let mut manager = PlatformManager::default();
    let root = PathBuf::from(".");
    let proto = Arc::new(ProtoEnvironment::new_testing(&root));

    manager.register(
        PlatformType::Node,
        Box::new(NodePlatform::new(
            &NodeConfig {
                version: Some(UnresolvedVersionSpec::Version(Version::new(20, 0, 0))),
                ..Default::default()
            },
            &None,
            &root,
            proto.clone(),
        )),
    );

    manager.register(
        PlatformType::Rust,
        Box::new(RustPlatform::new(
            &RustConfig {
                version: Some(UnresolvedVersionSpec::Version(Version::new(1, 72, 0))),
                ..Default::default()
            },
            &root,
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

    mod setup_tool {
        use super::*;

        #[test]
        fn graphs() {
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

        #[test]
        fn graphs_same_platform() {
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

        #[test]
        fn ignores_dupes() {
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

        #[test]
        fn graphs_single() {
            let pg = create_project_graph();
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

        #[test]
        fn graphs_single_with_dep() {
            let pg = create_project_graph();
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

        #[test]
        fn graphs_multiple() {
            let pg = create_project_graph();
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

        #[test]
        fn ignores_dupes() {
            let pg = create_project_graph();
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

        #[test]
        fn inherits_platform_tool() {
            let pm = create_platform_manager();
            let pg = create_project_graph();
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
                        runtime: Runtime::new(
                            PlatformType::Rust,
                            RuntimeReq::with_version(Version::new(1, 72, 0))
                        )
                    },
                    ActionNode::SetupTool {
                        runtime: Runtime::new(
                            PlatformType::Node,
                            RuntimeReq::with_version(Version::new(20, 0, 0))
                        )
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("qux"),
                        runtime: Runtime::new(
                            PlatformType::Rust,
                            RuntimeReq::with_version(Version::new(1, 72, 0))
                        )
                    },
                    ActionNode::SyncProject {
                        project: Id::raw("bar"),
                        runtime: Runtime::new(
                            PlatformType::Node,
                            RuntimeReq::with_version(Version::new(20, 0, 0))
                        )
                    }
                ]
            );
        }

        #[test]
        fn supports_platform_override() {
            let pm = create_platform_manager();
            let pg = create_project_graph();
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
                        runtime: Runtime::new(
                            PlatformType::Node,
                            RuntimeReq::with_version(Version::new(20, 0, 0))
                        )
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
                        runtime: Runtime::new(
                            PlatformType::Node,
                            RuntimeReq::with_version(Version::new(20, 0, 0))
                        )
                    },
                ]
            );
        }
    }

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

        #[test]
        fn ignores_dupes() {
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
