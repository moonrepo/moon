#![allow(clippy::disallowed_names)]

mod utils;

use moon_action::*;
use moon_action_context::TargetState;
use moon_action_graph::*;
use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{TaskArgs, TaskDependencyConfig, TaskOptionRunInCI};
use moon_platform::*;
use moon_task::{Target, TargetLocator, Task};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{assert_snapshot, create_sandbox};
use std::env;
use utils::ActionGraphContainer;

fn create_task(id: &str, project: &str) -> Task {
    Task {
        id: Id::raw(id),
        target: Target::new(project, id).unwrap(),
        toolchains: vec![Id::raw("node")],
        ..Task::default()
    }
}

fn create_runtime_with_version(version: Version) -> RuntimeReq {
    RuntimeReq::Toolchain(UnresolvedVersionSpec::Semantic(SemVer(version)))
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

fn topo(graph: ActionGraph) -> Vec<ActionNode> {
    let mut nodes = vec![];

    for index in graph.sort_topological().unwrap() {
        nodes.push(graph.get_node_from_index(&index).unwrap().to_owned());
    }

    nodes
}

mod action_graph {
    use super::*;

    #[tokio::test]
    #[should_panic(expected = "A dependency cycle has been detected for RunTask(deps:cycle2) →")]
    async fn errors_on_cycle() {
        let sandbox = create_sandbox("tasks");
        let container = ActionGraphContainer::new(sandbox.path()).await;
        let mut builder = container.create_builder();

        let project = container.workspace_graph.get_project("deps").unwrap();

        builder
            .run_task(
                &project,
                &container
                    .workspace_graph
                    .get_task_from_project(&project.id, "cycle1")
                    .unwrap(),
                &RunRequirements::default(),
            )
            .unwrap();
        builder
            .run_task(
                &project,
                &container
                    .workspace_graph
                    .get_task_from_project(&project.id, "cycle2")
                    .unwrap(),
                &RunRequirements::default(),
            )
            .unwrap();

        builder.build().sort_topological().unwrap();
    }

    mod run_task {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

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
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target, create_node_runtime()))
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();
            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();
            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

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
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target, create_node_runtime()))
                ]
            );
        }

        #[tokio::test]
        async fn task_can_have_a_diff_toolchain_from_project() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            // node
            let project = container.workspace_graph.get_project("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.toolchains = vec![Id::raw("rust")];

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain_legacy(SetupToolchainLegacyNode {
                        runtime: create_rust_runtime()
                    }),
                    ActionNode::install_project_deps(InstallProjectDepsNode {
                        project_id: Id::raw("bar"),
                        runtime: create_rust_runtime()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target, create_rust_runtime()))
                ]
            );
        }

        #[tokio::test]
        async fn sets_interactive() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.options.interactive = true;

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target, create_node_runtime());
                    node.interactive = true;
                    node
                })
            );
        }

        #[tokio::test]
        async fn sets_interactive_from_requirement() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            builder
                .run_task(
                    &project,
                    &task,
                    &RunRequirements {
                        interactive: true,
                        ..Default::default()
                    },
                )
                .unwrap();

            let graph = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target, create_node_runtime());
                    node.interactive = true;
                    node
                })
            );
        }

        #[tokio::test]
        async fn sets_persistent() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();

            let mut task = create_task("build", "bar");
            task.options.persistent = true;

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target, create_node_runtime());
                    node.persistent = true;
                    node
                })
            );
        }

        #[tokio::test]
        async fn distinguishes_between_args() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            // Test collapsing
            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();
            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::List(vec!["x".into(), "y".into(), "z".into()]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();

            let graph = builder.build();

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
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task(RunTaskNode::new(
                        task.target.clone(),
                        create_node_runtime()
                    )),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone(), create_node_runtime());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime());
                        node.args = vec!["x".into(), "y".into(), "z".into()];
                        node
                    })
                ]
            );
        }

        #[tokio::test]
        async fn flattens_same_args() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();

            let graph = builder.build();

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
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                ]
            );
        }

        #[tokio::test]
        async fn flattens_same_args_with_diff_enum() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::List(vec!["a".into(), "b".into(), "c".into()]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();

            let graph = builder.build();

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
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                ]
            );
        }

        #[tokio::test]
        async fn distinguishes_between_env() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            // Test collapsing
            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();
            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        env: FxHashMap::from_iter([("FOO".into(), "1".into())]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        env: FxHashMap::from_iter([("BAR".into(), "2".into())]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();

            let graph = builder.build();

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
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task(RunTaskNode::new(
                        task.target.clone(),
                        create_node_runtime()
                    )),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone(), create_node_runtime());
                        node.env = FxHashMap::from_iter([("FOO".into(), "1".into())]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime());
                        node.env = FxHashMap::from_iter([("BAR".into(), "2".into())]);
                        node
                    })
                ]
            );
        }

        #[tokio::test]
        async fn flattens_same_env() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        env: FxHashMap::from_iter([("FOO".into(), "1".into())]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        env: FxHashMap::from_iter([("FOO".into(), "1".into())]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();

            let graph = builder.build();

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
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime());
                        node.env = FxHashMap::from_iter([("FOO".into(), "1".into())]);
                        node
                    }),
                ]
            );
        }

        #[tokio::test]
        async fn distinguishes_between_args_and_env() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("bar").unwrap();
            let task = create_task("build", "bar");

            // Test collapsing
            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();
            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        env: FxHashMap::from_iter([("FOO".into(), "1".into())]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        env: FxHashMap::from_iter([("BAR".into(), "2".into())]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();
            builder
                .run_task_with_config(
                    &project,
                    &task,
                    &RunRequirements::default(),
                    Some(&TaskDependencyConfig {
                        args: TaskArgs::String("x y z".into()),
                        env: FxHashMap::from_iter([("BAR".into(), "2".into())]),
                        ..TaskDependencyConfig::default()
                    }),
                )
                .unwrap();

            let graph = builder.build();

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
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::run_task(RunTaskNode::new(
                        task.target.clone(),
                        create_node_runtime()
                    )),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone(), create_node_runtime());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node.env = FxHashMap::from_iter([("FOO".into(), "1".into())]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone(), create_node_runtime());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node.env = FxHashMap::from_iter([("BAR".into(), "2".into())]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target, create_node_runtime());
                        node.args = vec!["x".into(), "y".into(), "z".into()];
                        node.env = FxHashMap::from_iter([("BAR".into(), "2".into())]);
                        node
                    }),
                ]
            );
        }

        mod affected {
            use super::*;
            use moon_affected::AffectedBy;

            #[tokio::test]
            async fn doesnt_graph_if_not_affected_by_touched_files() {
                let sandbox = create_sandbox("projects");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("bar").unwrap();
                let task = create_task("build", "bar");

                // Empty set works fine, just needs to be some
                let touched_files = FxHashSet::default();
                builder.set_touched_files(touched_files).unwrap();
                builder.set_affected();

                builder
                    .run_task(&project, &task, &RunRequirements::default())
                    .unwrap();

                let graph = builder.build();

                assert!(!topo(graph).into_iter().any(|node| {
                    if let ActionNode::RunTask(inner) = &node {
                        inner.target.as_str() == "bar:build"
                    } else {
                        false
                    }
                }));
            }

            #[tokio::test]
            async fn graphs_if_affected_by_touched_files() {
                let sandbox = create_sandbox("projects");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let file = WorkspaceRelativePathBuf::from("bar/file.js");

                let project = container.workspace_graph.get_project("bar").unwrap();

                let mut task = create_task("build", "bar");
                task.input_files.insert(file.clone());

                let touched_files = FxHashSet::from_iter([file]);
                builder.set_touched_files(touched_files).unwrap();
                builder.set_affected();
                builder.mock_affected(|affected| {
                    affected
                        .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                        .unwrap();
                });

                builder
                    .run_task(&project, &task, &RunRequirements::default())
                    .unwrap();

                let graph = builder.build();

                assert!(!topo(graph).is_empty());
            }

            #[tokio::test]
            async fn includes_deps_if_owning_task_is_affected() {
                let sandbox = create_sandbox("tasks");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container
                    .workspace_graph
                    .get_project("deps-affected")
                    .unwrap();
                let task = container
                    .workspace_graph
                    .get_task_from_project("deps-affected", "b")
                    .unwrap();

                let touched_files =
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]);
                builder.set_touched_files(touched_files).unwrap();
                builder.set_affected();
                builder.mock_affected(|affected| {
                    affected
                        .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                        .unwrap();
                });

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            dependents: true,
                            ..Default::default()
                        },
                    )
                    .unwrap();

                let graph = builder.build();

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
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:a").unwrap(),
                            Runtime::system()
                        )),
                    ]
                );
            }
        }

        mod run_in_ci {
            use super::*;

            #[tokio::test]
            async fn graphs_if_ci_check_true() {
                let sandbox = create_sandbox("projects");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("bar").unwrap();

                let mut task = create_task("build", "bar");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(true);

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            ..Default::default()
                        },
                    )
                    .unwrap();

                let context = builder.build_context();
                let graph = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            #[tokio::test]
            async fn doesnt_run_dependents_if_its_ci_false() {
                let sandbox = create_sandbox("tasks");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("ci").unwrap();
                let task = container
                    .workspace_graph
                    .get_task_from_project(&project.id, "ci3-dependency")
                    .unwrap();

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: true,
                            ..RunRequirements::default()
                        },
                    )
                    .unwrap();

                let graph = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test]
            async fn runs_dependents_if_both_are_ci_true() {
                let sandbox = create_sandbox("tasks");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("ci").unwrap();
                let task = container
                    .workspace_graph
                    .get_task_from_project(&project.id, "ci4-dependency")
                    .unwrap();

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: true,
                            ..RunRequirements::default()
                        },
                    )
                    .unwrap();

                let graph = builder.build();

                assert_snapshot!(graph.to_dot());
            }
        }

        mod dont_run_in_ci {
            use super::*;

            #[tokio::test]
            async fn doesnt_graph_if_ci_check_true() {
                let sandbox = create_sandbox("projects");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("bar").unwrap();

                let mut task = create_task("build", "bar");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            ..Default::default()
                        },
                    )
                    .unwrap();

                let context = builder.build_context();
                let graph = builder.build();

                assert_eq!(
                    context.get_target_states(),
                    FxHashMap::from_iter([(
                        Target::parse("bar:build").unwrap(),
                        TargetState::Passthrough
                    )])
                );

                assert!(topo(graph).is_empty());
            }

            #[tokio::test]
            async fn graphs_if_ci_check_false() {
                let sandbox = create_sandbox("projects");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("bar").unwrap();

                let mut task = create_task("build", "bar");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: false,
                            ..Default::default()
                        },
                    )
                    .unwrap();

                let context = builder.build_context();
                let graph = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            #[tokio::test]
            async fn graphs_if_ci_false() {
                let sandbox = create_sandbox("projects");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("bar").unwrap();

                let mut task = create_task("build", "bar");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            ci: false,
                            ci_check: false,
                            ..Default::default()
                        },
                    )
                    .unwrap();

                let context = builder.build_context();
                let graph = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            // TODO: Enable after new task graph!
            // #[tokio::test]
            // async fn runs_dependents_if_dependency_is_ci_false_but_affected() {
            //     let sandbox = create_sandbox("tasks");
            //     let container = ActionGraphContainer::new(sandbox.path()).await;
            //     let mut builder = container.create_builder();

            //     let project = container.workspace_graph.get_project("ci").unwrap();
            //     let task = project.get_task("ci2-dependency").unwrap();

            //     // Must be affected to run the dependent
            //     let touched_files =
            //         FxHashSet::from_iter([WorkspaceRelativePathBuf::from("ci/input.txt")]);

            //     builder.set_touched_files(&touched_files).unwrap();

            //     builder
            //         .run_task(
            //             &project,
            //             task,
            //             &RunRequirements {
            //                 ci: true,
            //                 ci_check: true,
            //                 dependents: true,
            //                 ..RunRequirements::default()
            //             },
            //         )
            //         .unwrap();

            //     let graph = builder.build();

            //     assert_snapshot!(graph.to_dot());
            // }

            #[tokio::test]
            async fn doesnt_run_dependents_if_dependency_is_ci_false_and_not_affected() {
                let sandbox = create_sandbox("tasks");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("ci").unwrap();
                let task = container
                    .workspace_graph
                    .get_task_from_project(&project.id, "ci2-dependency")
                    .unwrap();

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: true,
                            ..RunRequirements::default()
                        },
                    )
                    .unwrap();

                let graph = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test]
            async fn doesnt_run_dependents_if_both_are_ci_false() {
                let sandbox = create_sandbox("tasks");
                let container = ActionGraphContainer::new(sandbox.path()).await;
                let mut builder = container.create_builder();

                let project = container.workspace_graph.get_project("ci").unwrap();
                let task = container
                    .workspace_graph
                    .get_task_from_project(&project.id, "ci2-dependency")
                    .unwrap();

                builder
                    .run_task(
                        &project,
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: true,
                            ..RunRequirements::default()
                        },
                    )
                    .unwrap();

                let graph = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test]
            #[should_panic(
                expected = "Task ci:ci1-dependent cannot depend on task ci:ci1-dependency"
            )]
            async fn errors_if_dependency_is_ci_false_and_constraint_enabled() {
                let sandbox = create_sandbox("tasks-ci-mismatch");
                ActionGraphContainer::new(sandbox.path()).await;
            }
        }
    }
}
