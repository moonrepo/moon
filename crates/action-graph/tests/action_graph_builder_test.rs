mod utils;

use moon_action::*;
use moon_action_context::TargetState;
use moon_action_graph::{ActionGraph, ActionGraphBuilderOptions, RunRequirements};
use moon_affected::{AffectedBy, DownstreamScope, UpstreamScope};
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::{
    EnvMap, PipelineActionSwitch, SemVer, TaskDependencyConfig, TaskOptionRunInCI,
    UnresolvedVersionSpec, Version, VersionSpec,
};
use moon_graph_utils::*;
use moon_task::{Target, TargetLocator, Task, TaskFileInput};
use moon_toolchain::ToolchainSpec;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{assert_snapshot, create_sandbox};
use utils::ActionGraphContainer;

fn create_task(project: &str, id: &str) -> Task {
    Task {
        id: Id::raw(id),
        target: Target::new(project, id).unwrap(),
        toolchains: vec![Id::raw("node")],
        ..Task::default()
    }
}

fn create_proto_version() -> VersionSpec {
    VersionSpec::parse("0.55.2").unwrap()
}

fn create_unresolved_version(version: Version) -> UnresolvedVersionSpec {
    UnresolvedVersionSpec::Semantic(SemVer(version))
}

fn create_node_spec() -> ToolchainSpec {
    ToolchainSpec::new(
        Id::raw("node"),
        UnresolvedVersionSpec::parse("20.0.0").unwrap(),
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

fn topo(graph: ActionGraph) -> Vec<ActionNode> {
    let mut nodes = vec![];

    for index in graph.sort_topological().unwrap() {
        nodes.push(graph.get_node_from_index(&index).unwrap().to_owned());
    }

    nodes
}

mod action_graph_builder {
    use super::*;

    // #[tokio::test(flavor = "multi_thread")]
    // #[should_panic(expected = "A dependency cycle has been detected for RunTask(deps:cycle2) →")]
    // async fn errors_on_cycle() {
    //     let sandbox = create_sandbox("tasks");
    //     let mut container = ActionGraphContainer2::new(sandbox.path());

    //     let wg = container.create_workspace_graph().await;
    //     let mut builder = container.create_builder(wg.clone()).await;

    //     builder
    //         .run_task(
    //             &wg.get_task_from_project("deps", "cycle1").unwrap(),
    //             &RunRequirements::default(),
    //         )
    //         .await
    //         .unwrap();
    //     builder
    //         .run_task(
    //             &wg.get_task_from_project("deps", "cycle2").unwrap(),
    //             &RunRequirements::default(),
    //         )
    //         .await
    //         .unwrap();

    //     let (_, ag) = builder.build();

    //     ag.sort_topological().unwrap();
    // }

    mod install_deps {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_toolchain_if_tier3() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: spec.clone()
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_multiple_toolchain_versions() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec1 = create_tier_spec(3);

            let mut spec2 = create_tier_spec(3);
            spec2.req = Some(create_unresolved_version(Version::new(4, 5, 6)));

            let project = wg.get_project("bar").unwrap();
            builder
                .install_dependencies(&spec1, &project)
                .await
                .unwrap();
            builder
                .install_dependencies(&spec2, &project)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: spec1.clone()
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: spec2 }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec1.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_multiple_toolchain_versions_using_overrides() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let project = wg.get_project("bar").unwrap();
            builder
                .install_dependencies(
                    &builder
                        .get_project_spec(&Id::raw("rust"), &project)
                        .unwrap(),
                    &project,
                )
                .await
                .unwrap();

            let project = wg.get_project("qux").unwrap();
            builder
                .install_dependencies(
                    &builder
                        .get_project_spec(&Id::raw("rust"), &project)
                        .unwrap(),
                    &project,
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("rust"),
                            UnresolvedVersionSpec::parse("1.70.0").unwrap()
                        )
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("rust"),
                            UnresolvedVersionSpec::parse("1.90.0").unwrap()
                        )
                    }),
                    // No install dependencies because `Cargo.toml`
                    // is not setup in the fixture!
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_env_chain_if_defined() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_project_if_in_project_root() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path())
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
                        members: None,
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
            let mut container = ActionGraphContainer::new(sandbox.path())
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
                        members: None,
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
            let mut container = ActionGraphContainer::new(sandbox.path())
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
                        members: Some(vec!["in".into()]),
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
            let mut container = ActionGraphContainer::new(sandbox.path())
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
                        members: Some(vec!["in".into()]),
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }
    }

    mod run_task {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone()))
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone()))
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_interactive() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let mut task = create_task("bar", "build");
            task.options.interactive = true;

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target);
                    node.interactive = true;
                    node
                })
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_interactive_from_requirement() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task(
                    &task,
                    &RunRequirements {
                        interactive: true,
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target);
                    node.interactive = true;
                    node
                })
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_persistent() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let mut task = create_task("bar", "build");
            task.options.persistent = true;

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target);
                    node.persistent = true;
                    node
                })
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn distinguishes_between_args() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            // Test collapsing
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["x".into(), "y".into(), "z".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone())),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.args = vec!["x".into(), "y".into(), "z".into()];
                        node
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn flattens_same_args() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn flattens_same_args_with_diff_enum() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn distinguishes_between_env() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            // Test collapsing
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone())),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone());
                        node.env = EnvMap::from_iter([("FOO".into(), Some("1".into()))]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.env = EnvMap::from_iter([("BAR".into(), Some("2".into()))]);
                        node
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn flattens_same_env() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.env = EnvMap::from_iter([("FOO".into(), Some("1".into()))]);
                        node
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn distinguishes_between_args_and_env() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            // Test collapsing
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["x".into(), "y".into(), "z".into()],
                        env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone())),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node.env = EnvMap::from_iter([("FOO".into(), Some("1".into()))]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node.env = EnvMap::from_iter([("BAR".into(), Some("2".into()))]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.args = vec!["x".into(), "y".into(), "z".into()];
                        node.env = EnvMap::from_iter([("BAR".into(), Some("2".into()))]);
                        node
                    }),
                ]
            );
        }

        mod affected {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_graph_if_not_affected_by_changed_files() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let task = create_task("bar", "build");

                // Empty set works fine, just needs to be some
                let changed_files = FxHashSet::default();
                builder.set_changed_files(changed_files).unwrap();
                builder.set_affected().unwrap();

                builder
                    .run_task(&task, &RunRequirements::default())
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert!(!topo(graph).into_iter().any(|node| {
                    if let ActionNode::RunTask(inner) = &node {
                        inner.target.as_str() == "bar:build"
                    } else {
                        false
                    }
                }));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn graphs_if_affected_by_changed_files() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let file = WorkspaceRelativePathBuf::from("bar/file.js");

                let mut task = create_task("bar", "build");
                task.input_files
                    .insert(file.clone(), TaskFileInput::default());

                builder.mock_affected(FxHashSet::from_iter([file]), |affected| {
                    affected
                        .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                        .unwrap();
                });

                builder
                    .run_task(&task, &RunRequirements::default())
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert!(!topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn includes_deps_if_owning_task_is_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    |affected| {
                        affected.with_scopes(UpstreamScope::Deep, DownstreamScope::Deep);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependents: DownstreamScope::Deep,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_eq!(
                    topo(graph),
                    vec![
                        ActionNode::sync_workspace(),
                        ActionNode::sync_project(SyncProjectNode {
                            project_id: Id::raw("deps-affected"),
                        }),
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:d").unwrap(),
                        )),
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:c").unwrap(),
                        )),
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:b").unwrap(),
                        )),
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:a").unwrap(),
                        )),
                    ]
                );
            }
        }

        mod run_in_ci {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn graphs_if_ci_check_true() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(true);

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_dependents_if_its_ci_false() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci3-dependency").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_dependents_if_both_are_ci_true() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci4-dependency").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }
        }

        mod dont_run_in_ci {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_graph_if_task_ci_skip() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Skip;

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(
                    context.get_target_states(),
                    FxHashMap::from_iter([(
                        Target::parse("bar:build").unwrap(),
                        TargetState::Passthrough
                    )])
                );

                assert!(topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_graph_if_ci_check_true() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(
                    context.get_target_states(),
                    FxHashMap::from_iter([(
                        Target::parse("bar:build").unwrap(),
                        TargetState::Passthrough
                    )])
                );

                assert!(topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn graphs_if_ci_check_false() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: false,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn graphs_if_ci_false() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: false,
                            ci_check: false,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_dependents_if_dependency_is_ci_false_and_not_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci2-dependency").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_dependents_if_both_are_ci_false() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci2-dependency").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(
                expected = "Task ci:ci1-dependent cannot depend on task ci:ci1-dependency"
            )]
            async fn errors_if_dependency_is_ci_false_and_constraint_enabled() {
                let sandbox = create_sandbox("tasks-ci-mismatch");
                let mut container = ActionGraphContainer::new(sandbox.path());

                container
                    .create_builder(container.create_workspace_graph().await)
                    .await;
            }
        }

        mod dependencies {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_none_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_none_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    |affected| {
                        affected.with_scopes(UpstreamScope::None, DownstreamScope::None);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Direct,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    |affected| {
                        affected.with_scopes(UpstreamScope::Direct, DownstreamScope::None);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Direct,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    |affected| {
                        affected.with_scopes(UpstreamScope::Deep, DownstreamScope::None);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }
        }

        mod dependents {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_none_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_none_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "c").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/c.txt")]),
                    |affected| {
                        affected.with_scopes(UpstreamScope::None, DownstreamScope::None);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::Direct,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "c").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/c.txt")]),
                    |affected| {
                        affected.with_scopes(UpstreamScope::None, DownstreamScope::Direct);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::Direct,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "c").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/c.txt")]),
                    |affected| {
                        affected.with_scopes(UpstreamScope::None, DownstreamScope::Deep);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }
        }
    }

    mod run_task_dependencies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_deps_in_parallel() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "parallel").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:parallel").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_deps_in_serial() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "serial").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:serial").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_create_a_chain() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "chain1").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:chain1").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_include_dependents() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "base").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:base").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_dependents() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "base").unwrap();

            builder
                .run_task(
                    &task,
                    &RunRequirements {
                        dependents: DownstreamScope::Deep,
                        ..RunRequirements::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:base").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_dependents_for_ci() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "base").unwrap();

            builder
                .run_task(
                    &task,
                    &RunRequirements {
                        ci: true,
                        ci_check: true,
                        dependents: DownstreamScope::Deep,
                        ..RunRequirements::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:base").unwrap()]
            );
        }
    }

    mod run_task_by_target {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Dependencies scope (^:) is not supported in run contexts.")]
        async fn errors_on_parent_scope() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("^:build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Self scope (~:) is not supported in run contexts.")]
        async fn errors_on_self_scope() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("~:build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }
        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(
            expected = "No project has been configured with the identifier or alias unknown."
        )]
        async fn errors_for_unknown_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("unknown:build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unknown task unknown for project server.")]
        async fn errors_for_unknown_project_task() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("server:unknown").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unknown task internal for project common.")]
        async fn errors_for_internal_task_when_explicit() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("common:internal").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_all() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse(":build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("base:build").unwrap(),
                    Target::parse("common:build").unwrap(),
                    Target::parse("client:build").unwrap(),
                    Target::parse("server:build").unwrap(),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_all_with_query() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_query("language=rust").unwrap();

            builder
                .run_task_by_target(
                    Target::parse(":build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("server:build").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_all_no_nodes() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse(":unknown").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_run_all_internal() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse(":internal").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("client:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("client:lint").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_error_for_internal_task_when_depended_on() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("misc:requiresInternal").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("misc:requiresInternal").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_tag() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("#frontend:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("client:lint").unwrap(),
                    Target::parse("common:lint").unwrap()
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_tag_no_nodes() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("#unknown:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_run_tags_internal() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("#frontend:internal").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }
    }

    mod run_task_by_target_locator {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unknown task internal for project common.")]
        async fn errors_for_internal_task_when_explicit() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::Qualified(Target::parse("common:internal").unwrap()),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_target() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::Qualified(Target::parse("server:build").unwrap()),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("server:build").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_task_glob() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse(":*-dependency").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse(":{a,c}").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("deps:a").unwrap(),
                    Target::parse("ci:ci4-dependency").unwrap(),
                    Target::parse("deps-affected:a").unwrap(),
                    Target::parse("deps:c").unwrap(),
                    Target::parse("ci:ci2-dependency").unwrap(),
                    Target::parse("ci:ci3-dependency").unwrap(),
                    Target::parse("deps-affected:c").unwrap(),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_tag_glob() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse("#front*:build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("client:build").unwrap(),
                    Target::parse("common:build").unwrap(),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_project_glob() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse("c{lient,ommon}:test").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("client:test").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_empty_result_for_no_glob_match() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse("{foo,bar}:task-*").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_error_for_internal_task_when_depended_on() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::Qualified(Target::parse("misc:requiresInternal").unwrap()),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("misc:requiresInternal").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_in_default_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_workspace_config(|config| {
                config.default_project = Some(Id::raw("base"));
            });

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::DefaultProject(Id::raw("build")),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("base:build").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "No default project has been configured")]
        async fn errors_for_no_default_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::DefaultProject(Id::raw("build")),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Invalid default project")]
        async fn errors_for_invalid_default_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_workspace_config(|config| {
                config.default_project = Some(Id::raw("unknown"));
            });

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::DefaultProject(Id::raw("build")),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }
    }

    mod run_tasks {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_partition_if_no_job() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            // 0
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_tasks(
                    vec![TargetLocator::parse("partition:task-*").unwrap()],
                    RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, _) = builder.build();

            assert_eq!(context.primary_targets.len(), 10);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn partitions_by_job() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            // 0
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_tasks(
                    vec![TargetLocator::parse("partition:task-*").unwrap()],
                    RunRequirements {
                        job: Some(0),
                        job_total: Some(3),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_eq!(context.primary_targets.len(), 4);
            assert_snapshot!(graph.to_dot());

            // 1
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_tasks(
                    vec![TargetLocator::parse("partition:task-*").unwrap()],
                    RunRequirements {
                        job: Some(1),
                        job_total: Some(3),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_eq!(context.primary_targets.len(), 4);
            assert_snapshot!(graph.to_dot());

            // 2
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_tasks(
                    vec![TargetLocator::parse("partition:task-*").unwrap()],
                    RunRequirements {
                        job: Some(2),
                        job_total: Some(3),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_eq!(context.primary_targets.len(), 2);
            assert_snapshot!(graph.to_dot());
        }
    }

    mod setup_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let ts = ToolchainSpec::new_global(Id::raw("tc-tier1"));

            builder.setup_toolchain(&ts, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier2() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let ts = ToolchainSpec::new_global(Id::raw("tc-tier2"));

            builder.setup_toolchain(&ts, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_same_toolchain() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node1 = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );
            let node2 = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(4, 5, 6)),
            );
            let node3 = ToolchainSpec::new_global(Id::raw("tc-tier3"));
            let node4 = node1.clone();
            let node5 = node2.clone();

            builder.setup_toolchain(&node1, None).await.unwrap();
            builder.setup_toolchain(&node2, None).await.unwrap();
            builder.setup_toolchain(&node3, None).await.unwrap();
            builder.setup_toolchain(&node4, None).await.unwrap();
            builder.setup_toolchain(&node5, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node1 }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node2 }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node3 }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node = create_tier_spec(3);

            builder.setup_toolchain(&node, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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

            builder.setup_toolchain(&system, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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

            builder.setup_toolchain(&system, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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

            builder.setup_toolchain(&system, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_require_other_toolchains() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node = ToolchainSpec::new(
                Id::raw("tc-tier3-reqs"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_require_other_toolchains_when_no_setup_toolchain_itself() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node = ToolchainSpec::new(
                Id::raw("tc-tier2-reqs"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier2-reqs"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                ]
            );
        }
    }

    mod sync_project {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_single() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.sync_workspace().await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.sync_workspace().await.unwrap();
            builder.sync_workspace().await.unwrap();
            builder.sync_workspace().await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder_with_options(
                    container.create_workspace_graph().await,
                    ActionGraphBuilderOptions {
                        sync_workspace: false,
                        ..Default::default()
                    },
                )
                .await;

            builder.sync_workspace().await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }
}
