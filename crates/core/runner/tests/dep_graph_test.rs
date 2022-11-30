use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, NodeConfig, ToolchainConfig, WorkspaceConfig, WorkspaceProjects,
};
use moon_node_platform::NodePlatform;
use moon_platform::Platformable;
use moon_project_graph::ProjectGraph;
use moon_runner::{BatchedTopoSort, DepGraph, NodeIndex};
use moon_system_platform::SystemPlatform;
use moon_task::Target;
use moon_test_utils::{assert_snapshot, create_sandbox_with_config, Sandbox};
use moon_utils::string_vec;
use rustc_hash::{FxHashMap, FxHashSet};

fn register_platforms(project_graph: &mut ProjectGraph) {
    project_graph
        .register_platform(Box::new(NodePlatform::default()))
        .unwrap();
    project_graph
        .register_platform(Box::new(SystemPlatform::default()))
        .unwrap();
}

async fn create_project_graph() -> (ProjectGraph, Sandbox) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("advanced".to_owned(), "advanced".to_owned()),
            ("basic".to_owned(), "basic".to_owned()),
            ("emptyConfig".to_owned(), "empty-config".to_owned()),
            ("noConfig".to_owned(), "no-config".to_owned()),
            // Deps
            ("foo".to_owned(), "deps/foo".to_owned()),
            ("bar".to_owned(), "deps/bar".to_owned()),
            ("baz".to_owned(), "deps/baz".to_owned()),
            // Tasks
            ("tasks".to_owned(), "tasks".to_owned()),
            // Languages
            ("js".to_owned(), "langs/js".to_owned()),
            ("ts".to_owned(), "langs/ts".to_owned()),
            ("bash".to_owned(), "langs/bash".to_owned()),
            ("platforms".to_owned(), "platforms".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };
    let toolchain_config = ToolchainConfig {
        node: Some(NodeConfig {
            version: "16.0.0".into(),
            dedupe_on_lockfile_change: false,
            ..NodeConfig::default()
        }),
        ..ToolchainConfig::default()
    };
    let projects_config = GlobalProjectConfig {
        file_groups: FxHashMap::from_iter([
            ("sources".to_owned(), string_vec!["src/**/*", "types/**/*"]),
            ("tests".to_owned(), string_vec!["tests/**/*"]),
        ]),
        ..GlobalProjectConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let mut graph = ProjectGraph::generate(
        sandbox.path(),
        &workspace_config,
        &toolchain_config,
        projects_config,
        &CacheEngine::load(sandbox.path()).await.unwrap(),
    )
    .await
    .unwrap();

    register_platforms(&mut graph);

    (graph, sandbox)
}

async fn create_tasks_project_graph() -> (ProjectGraph, Sandbox) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("basic".to_owned(), "basic".to_owned()),
            ("buildA".to_owned(), "build-a".to_owned()),
            ("buildB".to_owned(), "build-b".to_owned()),
            ("buildC".to_owned(), "build-c".to_owned()),
            ("chain".to_owned(), "chain".to_owned()),
            ("cycle".to_owned(), "cycle".to_owned()),
            ("inputA".to_owned(), "input-a".to_owned()),
            ("inputB".to_owned(), "input-b".to_owned()),
            ("inputC".to_owned(), "input-c".to_owned()),
            (
                "mergeAllStrategies".to_owned(),
                "merge-all-strategies".to_owned(),
            ),
            ("mergeAppend".to_owned(), "merge-append".to_owned()),
            ("mergePrepend".to_owned(), "merge-prepend".to_owned()),
            ("mergeReplace".to_owned(), "merge-replace".to_owned()),
            ("noTasks".to_owned(), "no-tasks".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };
    let toolchain_config = ToolchainConfig {
        node: Some(NodeConfig {
            version: "16.0.0".into(),
            ..NodeConfig::default()
        }),
        ..ToolchainConfig::default()
    };
    let projects_config = GlobalProjectConfig {
        file_groups: FxHashMap::from_iter([("sources".to_owned(), vec!["src/**/*".to_owned()])]),
        ..GlobalProjectConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let mut graph = ProjectGraph::generate(
        sandbox.path(),
        &workspace_config,
        &toolchain_config,
        projects_config,
        &CacheEngine::load(sandbox.path()).await.unwrap(),
    )
    .await
    .unwrap();

    register_platforms(&mut graph);

    (graph, sandbox)
}

fn sort_batches(batches: BatchedTopoSort) -> BatchedTopoSort {
    let mut list: BatchedTopoSort = vec![];

    for batch in batches {
        let mut new_batch = batch.clone();
        new_batch.sort();
        list.push(new_batch);
    }

    list
}

#[test]
fn default_graph() {
    let graph = DepGraph::default();

    assert_snapshot!(graph.to_dot());

    assert_eq!(graph.sort_topological().unwrap(), vec![]);
}

#[tokio::test]
#[should_panic(
    expected = "CycleDetected(\"RunTarget(cycle:a) → RunTarget(cycle:b) → RunTarget(cycle:c)\")"
)]
async fn detects_cycles() {
    let (projects, _sandbox) = create_tasks_project_graph().await;

    let mut graph = DepGraph::default();
    graph
        .run_target(&Target::new("cycle", "a").unwrap(), &projects, None)
        .unwrap();
    graph
        .run_target(&Target::new("cycle", "b").unwrap(), &projects, None)
        .unwrap();
    graph
        .run_target(&Target::new("cycle", "c").unwrap(), &projects, None)
        .unwrap();

    assert_eq!(
        sort_batches(graph.sort_batched_topological().unwrap()),
        vec![vec![NodeIndex::new(0)], vec![NodeIndex::new(1)]]
    );
}

mod run_target {
    use super::*;

    #[tokio::test]
    async fn single_targets() {
        let (projects, _sandbox) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("tasks", "test").unwrap(), &projects, None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), &projects, None)
            .unwrap();
        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2), // sync project
                NodeIndex::new(3), // test
                NodeIndex::new(4), // lint
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(0)],
                vec![NodeIndex::new(1), NodeIndex::new(2)],
                vec![NodeIndex::new(3), NodeIndex::new(4)]
            ]
        );
    }

    #[tokio::test]
    async fn deps_chain_target() {
        let (projects, _sandbox) = create_tasks_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("basic", "test").unwrap(), &projects, None)
            .unwrap();
        graph
            .run_target(&Target::new("basic", "lint").unwrap(), &projects, None)
            .unwrap();
        graph
            .run_target(&Target::new("chain", "a").unwrap(), &projects, None)
            .unwrap();

        assert_snapshot!(graph.to_dot());
        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2),  // sync project
                NodeIndex::new(3),  // test
                NodeIndex::new(4),  // lint
                NodeIndex::new(5),  // sync project
                NodeIndex::new(11), // f
                NodeIndex::new(10), // e
                NodeIndex::new(9),  // d
                NodeIndex::new(8),  // c
                NodeIndex::new(7),  // b
                NodeIndex::new(6),  // a
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(0)],
                vec![NodeIndex::new(1), NodeIndex::new(5)],
                vec![NodeIndex::new(11)],
                vec![NodeIndex::new(10)],
                vec![NodeIndex::new(9)],
                vec![NodeIndex::new(8)],
                vec![NodeIndex::new(2), NodeIndex::new(7)],
                vec![NodeIndex::new(3), NodeIndex::new(4), NodeIndex::new(6)]
            ]
        );
    }

    #[tokio::test]
    async fn avoids_dupe_targets() {
        let (projects, _sandbox) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), &projects, None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), &projects, None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), &projects, None)
            .unwrap();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2), // sync project
                NodeIndex::new(3), // lint
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(0)],
                vec![NodeIndex::new(1), NodeIndex::new(2)],
                vec![NodeIndex::new(3)]
            ]
        );
    }

    #[tokio::test]
    async fn runs_all_projects_for_target_all_scope() {
        let (projects, _sandbox) = create_tasks_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::parse(":build").unwrap(), &projects, None)
            .unwrap();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2), // sync project: basic
                NodeIndex::new(3), // basic:build
                NodeIndex::new(5), // sync project: build-c
                NodeIndex::new(4), // sync project: build-a
                NodeIndex::new(7), // build-c:build
                NodeIndex::new(6), // build-a:build
                NodeIndex::new(8), // sync project: build-b
                NodeIndex::new(9), // build-b:build
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(0)],
                vec![NodeIndex::new(1), NodeIndex::new(2), NodeIndex::new(5)],
                vec![
                    NodeIndex::new(3),
                    NodeIndex::new(4),
                    NodeIndex::new(7),
                    NodeIndex::new(8)
                ],
                vec![NodeIndex::new(6), NodeIndex::new(9)],
            ]
        );
    }

    #[tokio::test]
    #[should_panic(expected = "Target(NoProjectDepsInRunContext)")]
    async fn errors_for_target_deps_scope() {
        let (projects, _sandbox) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::parse("^:lint").unwrap(), &projects, None)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Target(NoProjectSelfInRunContext)")]
    async fn errors_for_target_self_scope() {
        let (projects, _sandbox) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::parse("~:lint").unwrap(), &projects, None)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Project(UnconfiguredID(\"unknown\"))")]
    async fn errors_for_unknown_project() {
        let (projects, _sandbox) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("unknown", "test").unwrap(), &projects, None)
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    #[should_panic(expected = "Project(UnconfiguredTask(\"build\", \"tasks\"))")]
    async fn errors_for_unknown_task() {
        let (projects, _sandbox) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("tasks", "build").unwrap(), &projects, None)
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }
}

mod run_target_if_touched {
    use super::*;

    #[tokio::test]
    async fn skips_if_untouched_project() {
        let (projects, sandbox) = create_tasks_project_graph().await;

        let mut touched_files = FxHashSet::default();
        touched_files.insert(sandbox.path().join("input-a/a.ts"));
        touched_files.insert(sandbox.path().join("input-c/c.ts"));

        let mut graph = DepGraph::default();
        graph
            .run_target(
                &Target::new("inputA", "a").unwrap(),
                &projects,
                Some(&touched_files),
            )
            .unwrap();
        graph
            .run_target(
                &Target::new("inputB", "b").unwrap(),
                &projects,
                Some(&touched_files),
            )
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    async fn skips_if_untouched_task() {
        let (projects, sandbox) = create_tasks_project_graph().await;

        let mut touched_files = FxHashSet::default();
        touched_files.insert(sandbox.path().join("input-a/a2.ts"));
        touched_files.insert(sandbox.path().join("input-b/b2.ts"));
        touched_files.insert(sandbox.path().join("input-c/any.ts"));

        let mut graph = DepGraph::default();
        graph
            .run_target(
                &Target::new("inputA", "a").unwrap(),
                &projects,
                Some(&touched_files),
            )
            .unwrap();
        graph
            .run_target(
                &Target::new("inputB", "b2").unwrap(),
                &projects,
                Some(&touched_files),
            )
            .unwrap();
        graph
            .run_target(
                &Target::new("inputC", "c").unwrap(),
                &projects,
                Some(&touched_files),
            )
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }
}

mod sync_project {
    use super::*;

    fn sync_projects(graph: &mut DepGraph, projects: &ProjectGraph, ids: &[&str]) {
        for id in ids {
            let project = projects.load(id).unwrap();

            graph.sync_project(&project, projects).unwrap();
        }
    }

    #[tokio::test]
    async fn isolated_projects() {
        let (projects, _sandbox) = create_project_graph().await;
        let mut graph = DepGraph::default();

        sync_projects(
            &mut graph,
            &projects,
            &["advanced", "basic", "emptyConfig", "noConfig"],
        );

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1), // advanced
                NodeIndex::new(3), // noConfig
                NodeIndex::new(4),
                NodeIndex::new(2), // basic
                NodeIndex::new(5), // emptyConfig
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(3)],
                vec![NodeIndex::new(0), NodeIndex::new(4)],
                vec![NodeIndex::new(1), NodeIndex::new(2), NodeIndex::new(5)]
            ]
        );
    }

    #[tokio::test]
    async fn projects_with_deps() {
        let (projects, _sandbox) = create_project_graph().await;
        let mut graph = DepGraph::default();

        sync_projects(&mut graph, &projects, &["foo", "bar", "baz", "basic"]);

        // Not deterministic!
        // assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(2), // baz
                NodeIndex::new(3), // bar
                NodeIndex::new(4),
                NodeIndex::new(1), // foo
                NodeIndex::new(6), // noConfig
                NodeIndex::new(5), // basic
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(2)],
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(3),
                    NodeIndex::new(4),
                    NodeIndex::new(6)
                ],
                vec![NodeIndex::new(1), NodeIndex::new(5)]
            ]
        );
    }

    #[tokio::test]
    async fn projects_with_tasks() {
        let (projects, _sandbox) = create_project_graph().await;
        let mut graph = DepGraph::default();

        sync_projects(&mut graph, &projects, &["noConfig", "tasks"]);

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1), // noConfig
                NodeIndex::new(2),
                NodeIndex::new(3) // tasks
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(0), NodeIndex::new(2)],
                vec![NodeIndex::new(1), NodeIndex::new(3)]
            ]
        );
    }

    #[tokio::test]
    async fn avoids_dupe_projects() {
        let (projects, _sandbox) = create_project_graph().await;
        let mut graph = DepGraph::default();

        sync_projects(&mut graph, &projects, &["advanced", "advanced", "advanced"]);

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    #[should_panic(expected = "UnconfiguredID(\"unknown\")")]
    async fn errors_for_unknown_project() {
        let (projects, _sandbox) = create_project_graph().await;
        let mut graph = DepGraph::default();

        sync_projects(&mut graph, &projects, &["unknown"]);

        assert_snapshot!(graph.to_dot());
    }
}

mod installs_deps {
    use super::*;

    #[tokio::test]
    async fn tool_is_based_on_task_platform() {
        let (projects, _sandbox) = create_project_graph().await;
        let mut graph = DepGraph::default();

        graph
            .run_target(
                &Target::new("platforms", "system").unwrap(),
                &projects,
                None,
            )
            .unwrap();

        graph
            .run_target(&Target::new("platforms", "node").unwrap(), &projects, None)
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }
}
