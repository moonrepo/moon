mod utils;

use moon_action::*;
use moon_action_context::TargetState;
use moon_action_graph::{ActionGraph, ActionGraphBuilderOptions, RunRequirements};
use moon_affected::AffectedBy;
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::{
    PipelineActionSwitch, SemVer, TaskArgs, TaskDependencyConfig, TaskOptionRunInCI,
    UnresolvedVersionSpec, Version,
};
use moon_platform::{Runtime, RuntimeReq, ToolchainSpec};
use moon_task::{Target, TargetLocator, Task};
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

fn create_node_runtime_global() -> Runtime {
    Runtime::new(Id::raw("node"), RuntimeReq::Global)
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

    mod install_deps_legacy {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let runtime = create_node_runtime();

            let project = wg.get_project("bar").unwrap();
            builder
                .install_dependencies_legacy(&runtime, &project, false)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: runtime.clone(),
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime,
                        root: WorkspaceRelativePathBuf::new(),
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let runtime = create_node_runtime();

            let project = wg.get_project("bar").unwrap();
            builder
                .install_dependencies_legacy(&runtime, &project, false)
                .await
                .unwrap();
            builder
                .install_dependencies_legacy(&runtime, &project, false)
                .await
                .unwrap();
            builder
                .install_dependencies_legacy(&runtime, &project, false)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: runtime.clone(),
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime,
                        root: WorkspaceRelativePathBuf::new(),
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn installs_in_project_when_not_in_depman_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let runtime = create_node_runtime();

            let inside = wg.get_project("in").unwrap();
            builder
                .install_dependencies_legacy(&runtime, &inside, false)
                .await
                .unwrap();

            let outside = wg.get_project("out").unwrap();
            builder
                .install_dependencies_legacy(&runtime, &outside, false)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: runtime.clone(),
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: runtime.clone(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::install_project_deps(InstallProjectDepsNode {
                        project_id: Id::raw("out"),
                        runtime,
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_install_bun_and_node() {
            let sandbox = create_sandbox("projects");
            sandbox.append_file(".moon/toolchain.yml", "bun:\n  version: '1.0.0'");

            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;
            let project = wg.get_project("bar").unwrap();

            let bun = Runtime::new(
                Id::raw("bun"),
                create_runtime_with_version(Version::new(1, 0, 0)),
            );
            let node = create_node_runtime();

            builder
                .install_dependencies_legacy(&node, &project, true)
                .await
                .unwrap();
            builder
                .install_dependencies_legacy(&bun, &project, true)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: node.clone(),
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: node.clone(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: bun.clone(),
                    }),
                ]
            );

            // Reverse order
            let mut builder = container.create_builder(wg.clone()).await;
            let project = wg.get_project("bar").unwrap();

            builder
                .install_dependencies_legacy(&bun, &project, true)
                .await
                .unwrap();
            builder
                .install_dependencies_legacy(&node, &project, true)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode { runtime: bun }),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: node.clone()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: node,
                        root: WorkspaceRelativePathBuf::new(),
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

            let runtime = create_node_runtime();
            let project = wg.get_project("bar").unwrap();

            builder
                .install_dependencies_legacy(&runtime, &project, false)
                .await
                .unwrap();

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

            let runtime = create_node_runtime();
            let project = wg.get_project("bar").unwrap();

            builder
                .install_dependencies_legacy(&runtime, &project, false)
                .await
                .unwrap();

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
                        install_dependencies: PipelineActionSwitch::Only(vec![Id::raw("node")]),
                        ..Default::default()
                    },
                )
                .await;

            let runtime = create_node_runtime();
            let project = wg.get_project("bar").unwrap();

            builder
                .install_dependencies_legacy(&runtime, &project, false)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    })
                ]
            );
        }
    }

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
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime_global()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime_global(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(
                        task.target.clone(),
                        create_node_runtime_global()
                    ))
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
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime_global()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime_global(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(
                        task.target.clone(),
                        create_node_runtime_global()
                    ))
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
                    let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
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
                    let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
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
                    let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
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
                        args: TaskArgs::String("a b c".into()),
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
                        args: TaskArgs::List(vec!["x".into(), "y".into(), "z".into()]),
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
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime_global()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime_global(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(
                        task.target.clone(),
                        create_node_runtime_global()
                    )),
                    ActionNode::run_task({
                        let mut node =
                            RunTaskNode::new(task.target.clone(), create_node_runtime_global());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
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
                        args: TaskArgs::String("a b c".into()),
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
                        args: TaskArgs::String("a b c".into()),
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
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime_global()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime_global(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
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
                        args: TaskArgs::String("a b c".into()),
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
                        args: TaskArgs::List(vec!["a".into(), "b".into(), "c".into()]),
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
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime_global()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime_global(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
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
                        env: FxHashMap::from_iter([("FOO".into(), "1".into())]),
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
                        env: FxHashMap::from_iter([("BAR".into(), "2".into())]),
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
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime_global()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime_global(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(
                        task.target.clone(),
                        create_node_runtime_global()
                    )),
                    ActionNode::run_task({
                        let mut node =
                            RunTaskNode::new(task.target.clone(), create_node_runtime_global());
                        node.env = FxHashMap::from_iter([("FOO".into(), "1".into())]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
                        node.env = FxHashMap::from_iter([("BAR".into(), "2".into())]);
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
                        env: FxHashMap::from_iter([("FOO".into(), "1".into())]),
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
                        env: FxHashMap::from_iter([("FOO".into(), "1".into())]),
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
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime_global()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime_global(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
                        node.env = FxHashMap::from_iter([("FOO".into(), "1".into())]);
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
                        args: TaskArgs::String("a b c".into()),
                        env: FxHashMap::from_iter([("FOO".into(), "1".into())]),
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
                        args: TaskArgs::String("a b c".into()),
                        env: FxHashMap::from_iter([("BAR".into(), "2".into())]),
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
                        args: TaskArgs::String("x y z".into()),
                        env: FxHashMap::from_iter([("BAR".into(), "2".into())]),
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
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_node_runtime_global()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime_global(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(
                        task.target.clone(),
                        create_node_runtime_global()
                    )),
                    ActionNode::run_task({
                        let mut node =
                            RunTaskNode::new(task.target.clone(), create_node_runtime_global());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node.env = FxHashMap::from_iter([("FOO".into(), "1".into())]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node =
                            RunTaskNode::new(task.target.clone(), create_node_runtime_global());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node.env = FxHashMap::from_iter([("BAR".into(), "2".into())]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime_global());
                        node.args = vec!["x".into(), "y".into(), "z".into()];
                        node.env = FxHashMap::from_iter([("BAR".into(), "2".into())]);
                        node
                    }),
                ]
            );
        }

        mod affected {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_graph_if_not_affected_by_touched_files() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let task = create_task("bar", "build");

                // Empty set works fine, just needs to be some
                let touched_files = FxHashSet::default();
                builder.set_touched_files(touched_files).unwrap();
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
            async fn graphs_if_affected_by_touched_files() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let file = WorkspaceRelativePathBuf::from("bar/file.js");

                let mut task = create_task("bar", "build");
                task.input_files.insert(file.clone());

                let touched_files = FxHashSet::from_iter([file]);
                builder.set_touched_files(touched_files).unwrap();
                builder.set_affected().unwrap();
                builder.mock_affected(|affected| {
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

                let touched_files =
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]);
                builder.set_touched_files(touched_files).unwrap();
                builder.set_affected().unwrap();
                builder.mock_affected(|affected| {
                    affected
                        .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                        .unwrap();
                });

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependents: true,
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
                            Target::parse("deps-affected:c").unwrap(),
                            Runtime::system()
                        )),
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:b").unwrap(),
                            Runtime::system()
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
                            dependents: true,
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
                            dependents: true,
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
                            dependents: true,
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
                            dependents: true,
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
                        dependents: true,
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
                        dependents: true,
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
        #[should_panic(expected = "Unknown task internal for project common.")]
        async fn errors_for_internal_task_when_explicit_via_dir() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path())
                .set_working_dir(sandbox.path().join("common"));

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::TaskFromWorkingDir(Id::raw("internal")),
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
                [Target::parse("client:test").unwrap(),]
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
    }

    mod setup_toolchain_legacy {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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

            let system = Runtime::system();
            let node = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain_legacy(&system).await.unwrap();
            builder.setup_toolchain_legacy(&node).await.unwrap();

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

            let system = Runtime::system();
            let node = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain_legacy(&system).await.unwrap();
            builder.setup_toolchain_legacy(&node).await.unwrap();

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
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let ts = ToolchainSpec::new_global(Id::raw("tc-tier1"));

            builder.setup_toolchain(&ts).await.unwrap();

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

            builder.setup_toolchain(&ts).await.unwrap();

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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
            let mut container = ActionGraphContainer::new(sandbox.path());

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

            builder.setup_toolchain(&system).await.unwrap();
            builder.setup_toolchain(&node).await.unwrap();

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

            builder.setup_toolchain(&system).await.unwrap();
            builder.setup_toolchain(&node).await.unwrap();

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
