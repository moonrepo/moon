#![allow(clippy::disallowed_names)]

mod utils;

use moon_action::*;
use moon_action_context::TargetState;
use moon_action_graph::*;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{TaskArgs, TaskDependencyConfig, TaskOptionRunInCI};
use moon_platform::*;
use moon_task::{Target, TargetLocator, Task};
use moon_test_utils2::generate_workspace_graph;
use moon_workspace_graph::WorkspaceGraph;
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

async fn create_project_graph() -> WorkspaceGraph {
    generate_workspace_graph("projects").await
}

// fn create_bun_runtime() -> Runtime {
//     Runtime::new(
//         PlatformType::Bun,
//         create_runtime_with_version(Version::new(1, 0, 0)),
//     )
// }

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
    let mut iter = graph.create_iter(graph.sort_topological().unwrap());

    while iter.has_pending() {
        if let Some(index) = iter.next() {
            nodes.push(graph.get_node_from_index(&index).unwrap().to_owned());
            iter.mark_completed(index);
        }
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

    mod install_deps {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let bar = container.workspace_graph.get_project("bar").unwrap();
            builder.install_deps(&bar, None).unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    })
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let bar = container.workspace_graph.get_project("bar").unwrap();
            builder.install_deps(&bar, None).unwrap();
            builder.install_deps(&bar, None).unwrap();
            builder.install_deps(&bar, None).unwrap();

            let graph = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    })
                ]
            );
        }

        #[tokio::test]
        async fn installs_in_project_when_not_in_depman_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let inside = container.workspace_graph.get_project("in").unwrap();
            builder.install_deps(&inside, None).unwrap();

            let outside = container.workspace_graph.get_project("out").unwrap();
            builder.install_deps(&outside, None).unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::install_project_deps(InstallProjectDepsNode {
                        project_id: Id::raw("out"),
                        runtime: create_node_runtime()
                    }),
                ]
            );
        }

        #[tokio::test]
        async fn doesnt_install_bun_and_node() {
            let sandbox = create_sandbox("projects");
            sandbox.append_file(".moon/toolchain.yml", "bun:\n  version: '1.0.0'");

            let container = ActionGraphContainer::new(sandbox.path()).await;

            let mut bun = create_task("bun", "bar");
            bun.toolchains = vec![Id::raw("bun")];

            let node = create_task("node", "bar");

            let mut builder = container.create_builder();
            let project = container.workspace_graph.get_project("bar").unwrap();

            builder.install_deps(&project, Some(&bun)).unwrap();
            builder.install_deps(&project, Some(&node)).unwrap();

            let graph = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    })
                ]
            );

            // Reverse order
            let mut builder = container.create_builder();
            let project = container.workspace_graph.get_project("bar").unwrap();

            builder.install_deps(&project, Some(&node)).unwrap();
            builder.install_deps(&project, Some(&bun)).unwrap();

            let graph = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_rust_runtime()
                    }),
                    ActionNode::install_project_deps(InstallProjectDepsNode {
                        project_id: Id::raw("bar"),
                        runtime: create_rust_runtime()
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                        runtime: create_node_runtime(),
                        root: WorkspaceRelativePathBuf::new(),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
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
                        ActionNode::setup_toolchain(SetupToolchainNode {
                            runtime: Runtime::system()
                        }),
                        ActionNode::sync_project(SyncProjectNode {
                            project_id: Id::raw("deps-affected"),
                            runtime: Runtime::system()
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

    mod run_task_dependencies {
        use super::*;

        #[tokio::test]
        async fn runs_deps_in_parallel() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("deps").unwrap();
            let task = container
                .workspace_graph
                .get_task_from_project(&project.id, "parallel")
                .unwrap();

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn runs_deps_in_serial() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("deps").unwrap();
            let task = container
                .workspace_graph
                .get_task_from_project(&project.id, "serial")
                .unwrap();

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn can_create_a_chain() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("deps").unwrap();
            let task = container
                .workspace_graph
                .get_task_from_project(&project.id, "chain1")
                .unwrap();

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn doesnt_include_dependents() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("deps").unwrap();
            let task = container
                .workspace_graph
                .get_task_from_project(&project.id, "base")
                .unwrap();

            builder
                .run_task(&project, &task, &RunRequirements::default())
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn includes_dependents() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("deps").unwrap();
            let task = container
                .workspace_graph
                .get_task_from_project(&project.id, "base")
                .unwrap();

            builder
                .run_task(
                    &project,
                    &task,
                    &RunRequirements {
                        dependents: true,
                        ..RunRequirements::default()
                    },
                )
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn includes_dependents_for_ci() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let project = container.workspace_graph.get_project("deps").unwrap();
            let task = container
                .workspace_graph
                .get_task_from_project(&project.id, "base")
                .unwrap();

            builder
                .run_task(
                    &project,
                    &task,
                    &RunRequirements {
                        ci: true,
                        dependents: true,
                        ..RunRequirements::default()
                    },
                )
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }
    }

    mod run_task_by_target {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "Dependencies scope (^:) is not supported in run contexts.")]
        async fn errors_on_parent_scope() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("^:build").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();
        }

        #[tokio::test]
        #[should_panic(expected = "Self scope (~:) is not supported in run contexts.")]
        async fn errors_on_self_scope() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("~:build").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();
        }

        #[tokio::test]
        async fn runs_all() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse(":build").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn runs_all_with_query() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder.set_query("language=rust").unwrap();

            builder
                .run_task_by_target(
                    Target::parse(":build").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn runs_all_no_nodes() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse(":unknown").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert!(graph.is_empty());
        }

        #[tokio::test]
        async fn doesnt_run_all_internal() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse(":internal").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert!(graph.is_empty());
        }

        #[tokio::test]
        async fn runs_by_project() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("client:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        #[should_panic(
            expected = "No project has been configured with the identifier or alias unknown."
        )]
        async fn errors_for_unknown_project() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("unknown:build").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();
        }

        #[tokio::test]
        #[should_panic(expected = "Unknown task unknown for project server.")]
        async fn errors_for_unknown_project_task() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("server:unknown").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();
        }

        #[tokio::test]
        #[should_panic(expected = "Unknown task internal for project common.")]
        async fn errors_for_internal_task_when_explicit() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let locator = TargetLocator::Qualified(Target::parse("common:internal").unwrap());

            builder
                .run_task_by_target(
                    Target::parse("common:internal").unwrap(),
                    &RunRequirements {
                        target_locators: FxHashSet::from_iter([locator]),
                        ..RunRequirements::default()
                    },
                )
                .unwrap();
        }

        #[tokio::test]
        async fn doesnt_error_for_internal_task_when_implicit() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("common:internal").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn doesnt_error_for_internal_task_when_depended_on() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let locator = TargetLocator::Qualified(Target::parse("common:internal").unwrap());

            builder
                .run_task_by_target(
                    Target::parse("misc:requiresInternal").unwrap(),
                    &RunRequirements {
                        target_locators: FxHashSet::from_iter([locator]),
                        ..RunRequirements::default()
                    },
                )
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn runs_tag() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("#frontend:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn runs_tag_no_nodes() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("#unknown:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert!(graph.is_empty());
        }

        #[tokio::test]
        async fn doesnt_run_tags_internal() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_task_by_target(
                    Target::parse("#frontend:internal").unwrap(),
                    &RunRequirements::default(),
                )
                .unwrap();

            let graph = builder.build();

            assert!(graph.is_empty());
        }
    }

    mod run_from_requirements {
        use super::*;

        #[tokio::test]
        async fn runs_by_target() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_from_requirements(RunRequirements {
                    target_locators: FxHashSet::from_iter([TargetLocator::Qualified(
                        Target::parse("server:build").unwrap(),
                    )]),
                    ..Default::default()
                })
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn runs_by_task_glob() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_from_requirements(RunRequirements {
                    target_locators: FxHashSet::from_iter([
                        TargetLocator::parse(":*-dependency").unwrap(),
                        TargetLocator::parse(":{a,c}").unwrap(),
                    ]),
                    ..Default::default()
                })
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn runs_by_tag_glob() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_from_requirements(RunRequirements {
                    target_locators: FxHashSet::from_iter([
                        TargetLocator::parse("#front*:build").unwrap()
                    ]),
                    ..Default::default()
                })
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn runs_by_project_glob() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_from_requirements(RunRequirements {
                    target_locators: FxHashSet::from_iter([TargetLocator::parse(
                        "c{lient,ommon}:test",
                    )
                    .unwrap()]),
                    ..Default::default()
                })
                .unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn returns_empty_result_for_no_glob_match() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            builder
                .run_from_requirements(RunRequirements {
                    target_locators: FxHashSet::from_iter([TargetLocator::parse(
                        "{foo,bar}:task-*",
                    )
                    .unwrap()]),
                    ..Default::default()
                })
                .unwrap();

            let graph = builder.build();

            assert!(graph.is_empty());
        }

        #[tokio::test]
        async fn computes_context() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let target = Target::parse("client:build").unwrap();

            builder
                .run_from_requirements(RunRequirements {
                    target_locators: FxHashSet::from_iter([TargetLocator::Qualified(
                        target.clone(),
                    )]),
                    ..Default::default()
                })
                .unwrap();

            let context = builder.build_context();

            assert_eq!(
                context.initial_targets,
                FxHashSet::from_iter([target.clone()])
            );
            assert_eq!(context.primary_targets, FxHashSet::from_iter([target]));
        }

        #[tokio::test]
        async fn computes_context_for_all() {
            let sandbox = create_sandbox("tasks");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let target = Target::parse(":build").unwrap();

            builder
                .run_from_requirements(RunRequirements {
                    target_locators: FxHashSet::from_iter([TargetLocator::Qualified(
                        target.clone(),
                    )]),
                    ..Default::default()
                })
                .unwrap();

            let context = builder.build_context();

            assert_eq!(
                context.initial_targets,
                FxHashSet::from_iter([target.clone()])
            );
            assert_eq!(
                context.primary_targets,
                FxHashSet::from_iter([
                    Target::parse("client:build").unwrap(),
                    Target::parse("common:build").unwrap(),
                    Target::parse("server:build").unwrap(),
                    Target::parse("base:build").unwrap(),
                ])
            );
        }
    }

    mod setup_toolchain {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let wg = WorkspaceGraph::default();
            let mut builder = ActionGraphBuilder::new(&wg).unwrap();
            let system = Runtime::system();
            let node = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system);
            builder.setup_toolchain(&node);

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode { runtime: system }),
                    ActionNode::setup_toolchain(SetupToolchainNode { runtime: node }),
                ]
            );
        }

        #[tokio::test]
        async fn graphs_same_toolchain() {
            let wg = WorkspaceGraph::default();
            let mut builder = ActionGraphBuilder::new(&wg).unwrap();

            let node1 = Runtime::new(
                Id::raw("node"),
                create_runtime_with_version(Version::new(1, 2, 3)),
            );
            let node2 = Runtime::new_override(
                Id::raw("node"),
                create_runtime_with_version(Version::new(4, 5, 6)),
            );
            let node3 = Runtime::new(Id::raw("node"), RuntimeReq::Global);

            builder.setup_toolchain(&node1);
            builder.setup_toolchain(&node2);
            builder.setup_toolchain(&node3);

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode { runtime: node1 }),
                    ActionNode::setup_toolchain(SetupToolchainNode { runtime: node2 }),
                    ActionNode::setup_toolchain(SetupToolchainNode { runtime: node3 }),
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let wg = WorkspaceGraph::default();
            let mut builder = ActionGraphBuilder::new(&wg).unwrap();
            let system = Runtime::system();

            builder.setup_toolchain(&system);
            builder.setup_toolchain(&system);

            let graph = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode { runtime: system }),
                ]
            );
        }
    }

    mod sync_project {
        use super::*;

        #[tokio::test]
        async fn graphs_single() {
            let wg = create_project_graph().await;
            let mut builder = ActionGraphBuilder::new(&wg).unwrap();

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: Runtime::system()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: Runtime::system()
                    })
                ]
            );
        }

        #[tokio::test]
        async fn graphs_single_with_dep() {
            let wg = create_project_graph().await;
            let mut builder = ActionGraphBuilder::new(&wg).unwrap();

            let foo = wg.get_project("foo").unwrap();
            builder.sync_project(&foo).unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: Runtime::system()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: Runtime::system()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                        runtime: Runtime::system()
                    })
                ]
            );
        }

        #[tokio::test]
        async fn graphs_multiple() {
            let wg = create_project_graph().await;
            let mut builder = ActionGraphBuilder::new(&wg).unwrap();

            let foo = wg.get_project("foo").unwrap();
            builder.sync_project(&foo).unwrap();

            let bar = wg.get_project("bar").unwrap();
            builder.sync_project(&bar).unwrap();

            let qux = wg.get_project("qux").unwrap();
            builder.sync_project(&qux).unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: Runtime::system()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: Runtime::system()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                        runtime: Runtime::system()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("qux"),
                        runtime: Runtime::system()
                    }),
                ]
            );
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let wg = create_project_graph().await;
            let mut builder = ActionGraphBuilder::new(&wg).unwrap();

            let foo = wg.get_project("foo").unwrap();

            builder.sync_project(&foo).unwrap();
            builder.sync_project(&foo).unwrap();
            builder.sync_project(&foo).unwrap();

            let graph = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: Runtime::system()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: Runtime::system()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                        runtime: Runtime::system()
                    })
                ]
            );
        }

        #[tokio::test]
        async fn inherits_toolchain_tool() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let bar = container.workspace_graph.get_project("bar").unwrap();
            builder.sync_project(&bar).unwrap();

            let qux = container.workspace_graph.get_project("qux").unwrap();
            builder.sync_project(&qux).unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_rust_runtime()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("qux"),
                        runtime: create_rust_runtime()
                    }),
                ]
            );
        }

        #[tokio::test]
        async fn supports_toolchain_override() {
            let sandbox = create_sandbox("projects");
            let container = ActionGraphContainer::new(sandbox.path()).await;
            let mut builder = container.create_builder();

            let bar = container.workspace_graph.get_project("bar").unwrap();
            builder.sync_project(&bar).unwrap();

            let baz = container.workspace_graph.get_project("baz").unwrap();
            builder.sync_project(&baz).unwrap();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: create_node_runtime()
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                        runtime: create_node_runtime()
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        runtime: Runtime::new_override(
                            Id::raw("node"),
                            create_runtime_with_version(Version::new(18, 0, 0))
                        )
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("baz"),
                        runtime: Runtime::new_override(
                            Id::raw("node"),
                            create_runtime_with_version(Version::new(18, 0, 0))
                        )
                    }),
                ]
            );
        }
    }

    mod sync_workspace {
        use super::*;

        #[tokio::test]
        async fn graphs() {
            let wg = WorkspaceGraph::default();

            let mut builder = ActionGraphBuilder::new(&wg).unwrap();
            builder.sync_workspace();

            let graph = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test]
        async fn ignores_dupes() {
            let wg = WorkspaceGraph::default();

            let mut builder = ActionGraphBuilder::new(&wg).unwrap();
            builder.sync_workspace();
            builder.sync_workspace();
            builder.sync_workspace();

            let graph = builder.build();

            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }
    }
}
