mod utils;

use moon_action::*;
use moon_action_graph::{ActionGraph, action_graph_builder2::*};
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::{PipelineActionSwitch, SemVer, UnresolvedVersionSpec, Version};
use moon_platform::{Runtime, RuntimeReq, ToolchainSpec};
use starbase_sandbox::{assert_snapshot, create_sandbox};
use utils::ActionGraphContainer2;

fn create_unresolved_version(version: Version) -> UnresolvedVersionSpec {
    UnresolvedVersionSpec::Semantic(SemVer(version))
}

fn create_runtime_with_version(version: Version) -> RuntimeReq {
    RuntimeReq::Toolchain(create_unresolved_version(version))
}

fn create_node_runtime() -> Runtime {
    Runtime::new(
        Id::raw("node"),
        create_runtime_with_version(Version::new(20, 0, 0)),
    )
}

fn create_rust_runtime() -> Runtime {
    Runtime::new(
        Id::raw("rust"),
        create_runtime_with_version(Version::new(1, 70, 0)),
    )
}

fn create_tier_spec(tier: u8) -> ToolchainSpec {
    create_tier_spec_with_name(format!("tc-tier{tier}"))
}

fn create_tier_spec_with_name(id: impl AsRef<str>) -> ToolchainSpec {
    ToolchainSpec::new(
        Id::raw(id.as_ref()),
        create_unresolved_version(Version::new(1, 2, 3)),
    )
}

fn create_rust_spec() -> ToolchainSpec {
    ToolchainSpec::new(
        Id::raw("rust"),
        create_unresolved_version(Version::new(1, 70, 0)),
    )
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

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "A dependency cycle has been detected for RunTask(deps:cycle2) →")]
    async fn errors_on_cycle() {
        let sandbox = create_sandbox("tasks");
        let mut container = ActionGraphContainer2::new(sandbox.path());

        let wg = container.create_workspace_graph().await;
        let mut builder = container.create_builder(wg.clone()).await;

        builder
            .run_task(
                &wg.get_task_from_project("deps", "cycle1").unwrap(),
                &RunRequirements::default(),
            )
            .await
            .unwrap();
        builder
            .run_task(
                &wg.get_task_from_project("deps", "cycle2").unwrap(),
                &RunRequirements::default(),
            )
            .await
            .unwrap();

        let (_, ag) = builder.build();

        ag.sort_topological().unwrap();
    }

    mod install_deps {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(1);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_if_tier2() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_toolchain_if_tier3() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(3);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode { spec: spec.clone() }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_env_chain_if_defined() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        install_dependencies: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        install_dependencies: PipelineActionSwitch::Only(vec![Id::raw("rust")]),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        install_dependencies: PipelineActionSwitch::Only(vec![Id::raw("tc-tier2")]),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_project_if_in_project_root() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer2::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("isolated"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("isolated").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        project_id: Some(Id::raw("isolated")),
                        root: WorkspaceRelativePathBuf::from("isolated"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_not_in_deps_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer2::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("out"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("out").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        project_id: Some(Id::raw("out")),
                        root: WorkspaceRelativePathBuf::from("out"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_in_deps_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer2::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("in"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("in").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_in_deps_workspace_if_root_level() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer2::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("in-root"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("root").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }
    }

    mod setup_toolchain_legacy {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let system = Runtime::system();
            let node = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain_legacy(&system).await.unwrap();
            builder.setup_toolchain_legacy(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode { runtime: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_same_toolchain() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node1 = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );
            let node2 = Runtime::new_override(
                Id::raw("node"),
                create_runtime_with_version(Version::new(4, 5, 6)),
            );
            let node3 = Runtime::new(Id::raw("node"), RuntimeReq::Global);

            builder.setup_toolchain_legacy(&node1).await.unwrap();
            builder.setup_toolchain_legacy(&node2).await.unwrap();
            builder.setup_toolchain_legacy(&node3).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode { runtime: node1 }),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode { runtime: node2 }),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode { runtime: node3 }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node = create_node_runtime();

            builder.setup_toolchain_legacy(&node).await.unwrap();
            builder.setup_toolchain_legacy(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode { runtime: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let system = Runtime::system();
            let node = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain_legacy(&system).await.unwrap();
            builder.setup_toolchain_legacy(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: PipelineActionSwitch::Only(vec![Id::raw("system")]),
                        ..Default::default()
                    },
                )
                .await;

            let system = Runtime::system();
            let node = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain_legacy(&system).await.unwrap();
            builder.setup_toolchain_legacy(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: PipelineActionSwitch::Only(vec![
                            Id::raw("system"),
                            Id::raw("node"),
                        ]),
                        ..Default::default()
                    },
                )
                .await;

            let system = Runtime::system();
            let node = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain_legacy(&system).await.unwrap();
            builder.setup_toolchain_legacy(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode { runtime: node }),
                ]
            );
        }
    }

    mod setup_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let ts = ToolchainSpec::new_global(Id::raw("tc-tier1"));

            builder.setup_toolchain(&ts).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier2() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let ts = ToolchainSpec::new_global(Id::raw("tc-tier2"));

            builder.setup_toolchain(&ts).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system).await.unwrap();
            builder.setup_toolchain(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode { spec: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_same_toolchain() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node1 = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );
            let node2 = ToolchainSpec::new_override(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(4, 5, 6)),
            );
            let node3 = ToolchainSpec::new_global(Id::raw("tc-tier3"));

            builder.setup_toolchain(&node1).await.unwrap();
            builder.setup_toolchain(&node2).await.unwrap();
            builder.setup_toolchain(&node3).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode { spec: node1 }),
                    ActionNode::setup_toolchain(SetupToolchainNode { spec: node2 }),
                    ActionNode::setup_toolchain(SetupToolchainNode { spec: node3 }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node = create_tier_spec(3);

            builder.setup_toolchain(&node).await.unwrap();
            builder.setup_toolchain(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode { spec: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system).await.unwrap();
            builder.setup_toolchain(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: PipelineActionSwitch::Only(vec![Id::raw("system")]),
                        ..Default::default()
                    },
                )
                .await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system).await.unwrap();
            builder.setup_toolchain(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: PipelineActionSwitch::Only(vec![
                            Id::raw("system"),
                            Id::raw("tc-tier3"),
                        ]),
                        ..Default::default()
                    },
                )
                .await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system).await.unwrap();
            builder.setup_toolchain(&node).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode { spec: node }),
                ]
            );
        }
    }

    mod sync_project {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_single() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).await.unwrap();

            let (_, graph) = builder.build();

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

        #[tokio::test(flavor = "multi_thread")]
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

            let (_, graph) = builder.build();

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

        #[tokio::test(flavor = "multi_thread")]
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

            let (_, graph) = builder.build();

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

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let foo = wg.get_project("foo").unwrap();

            builder.sync_project(&foo).await.unwrap();
            builder.sync_project(&foo).await.unwrap();
            builder.sync_project(&foo).await.unwrap();

            let (_, graph) = builder.build();

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

        #[tokio::test(flavor = "multi_thread")]
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

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
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

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
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

            let (_, graph) = builder.build();

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

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.sync_workspace().await;

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer2::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.sync_workspace().await;
            builder.sync_workspace().await;
            builder.sync_workspace().await;

            let (_, graph) = builder.build();

            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
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

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }
}
