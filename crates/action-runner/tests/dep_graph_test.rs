use insta::assert_snapshot;
use moon_action_runner::{BatchedTopoSort, DepGraph, NodeIndex};
use moon_cache::CacheEngine;
use moon_config::{GlobalProjectConfig, WorkspaceConfig};
use moon_project::{ProjectGraph, Target};
use moon_utils::test::get_fixtures_dir;
use std::collections::{HashMap, HashSet};

async fn create_project_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("projects");
    let workspace_config = WorkspaceConfig {
        projects: HashMap::from([
            ("advanced".to_owned(), "advanced".to_owned()),
            ("basic".to_owned(), "basic".to_owned()),
            ("emptyConfig".to_owned(), "empty-config".to_owned()),
            ("noConfig".to_owned(), "no-config".to_owned()),
            ("foo".to_owned(), "deps/foo".to_owned()),
            ("bar".to_owned(), "deps/bar".to_owned()),
            ("baz".to_owned(), "deps/baz".to_owned()),
            ("tasks".to_owned(), "tasks".to_owned()),
        ]),
        ..WorkspaceConfig::default()
    };

    ProjectGraph::create(
        &workspace_root,
        &workspace_config,
        GlobalProjectConfig::default(),
        &CacheEngine::create(&workspace_root).await.unwrap(),
    )
    .await
    .unwrap()
}

async fn create_tasks_project_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("tasks");
    let workspace_config = WorkspaceConfig {
        projects: HashMap::from([
            ("basic".to_owned(), "basic".to_owned()),
            ("build-a".to_owned(), "build-a".to_owned()),
            ("build-b".to_owned(), "build-b".to_owned()),
            ("build-c".to_owned(), "build-c".to_owned()),
            ("chain".to_owned(), "chain".to_owned()),
            ("cycle".to_owned(), "cycle".to_owned()),
            ("inputA".to_owned(), "input-a".to_owned()),
            ("inputB".to_owned(), "input-b".to_owned()),
            ("inputC".to_owned(), "input-c".to_owned()),
            ("mergeAppend".to_owned(), "merge-append".to_owned()),
            ("mergePrepend".to_owned(), "merge-prepend".to_owned()),
            ("mergeReplace".to_owned(), "merge-replace".to_owned()),
            ("no-tasks".to_owned(), "no-tasks".to_owned()),
        ]),
        ..WorkspaceConfig::default()
    };
    let global_config = GlobalProjectConfig {
        file_groups: HashMap::from([("sources".to_owned(), vec!["src/**/*".to_owned()])]),
        ..GlobalProjectConfig::default()
    };

    ProjectGraph::create(
        &workspace_root,
        &workspace_config,
        global_config,
        &CacheEngine::create(&workspace_root).await.unwrap(),
    )
    .await
    .unwrap()
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

    assert_eq!(graph.sort_topological().unwrap(), vec![NodeIndex::new(0)]);
    assert_eq!(
        sort_batches(graph.sort_batched_topological().unwrap()),
        vec![vec![NodeIndex::new(0)]]
    );
}

#[tokio::test]
#[should_panic(
    expected = "CycleDetected(\"RunTarget(cycle:a) → RunTarget(cycle:b) → RunTarget(cycle:c)\")"
)]
async fn detects_cycles() {
    let projects = create_tasks_project_graph().await;

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
        let projects = create_project_graph().await;

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
        let projects = create_tasks_project_graph().await;

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
        let projects = create_project_graph().await;

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
        let projects = create_tasks_project_graph().await;

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
    #[should_panic(expected = "Project(Target(NoProjectDepsInRunContext))")]
    async fn errors_for_target_deps_scope() {
        let projects = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::parse("^:lint").unwrap(), &projects, None)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Project(Target(NoProjectSelfInRunContext))")]
    async fn errors_for_target_self_scope() {
        let projects = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::parse("~:lint").unwrap(), &projects, None)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Project(UnconfiguredID(\"unknown\"))")]
    async fn errors_for_unknown_project() {
        let projects = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("unknown", "test").unwrap(), &projects, None)
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    #[should_panic(expected = "Project(UnconfiguredTask(\"build\", \"tasks\"))")]
    async fn errors_for_unknown_task() {
        let projects = create_project_graph().await;

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
        let projects = create_tasks_project_graph().await;

        let mut touched_files = HashSet::new();
        touched_files.insert(get_fixtures_dir("tasks").join("input-a/a.ts"));
        touched_files.insert(get_fixtures_dir("tasks").join("input-c/c.ts"));

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
        let projects = create_tasks_project_graph().await;

        let mut touched_files = HashSet::new();
        touched_files.insert(get_fixtures_dir("tasks").join("input-a/a2.ts"));
        touched_files.insert(get_fixtures_dir("tasks").join("input-b/b2.ts"));
        touched_files.insert(get_fixtures_dir("tasks").join("input-c/any.ts"));

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

    #[tokio::test]
    async fn isolated_projects() {
        let projects = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph.sync_project("advanced", &projects).unwrap();
        graph.sync_project("basic", &projects).unwrap();
        graph.sync_project("emptyConfig", &projects).unwrap();
        graph.sync_project("noConfig", &projects).unwrap();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1), // advanced
                NodeIndex::new(3), // noConfig
                NodeIndex::new(2), // basic
                NodeIndex::new(4), // emptyConfig
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(0)],
                vec![NodeIndex::new(3)],
                vec![NodeIndex::new(1), NodeIndex::new(2), NodeIndex::new(4)]
            ]
        );
    }

    #[tokio::test]
    async fn projects_with_deps() {
        let projects = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph.sync_project("foo", &projects).unwrap();
        graph.sync_project("bar", &projects).unwrap();
        graph.sync_project("baz", &projects).unwrap();
        graph.sync_project("basic", &projects).unwrap();

        // Not deterministic!
        // assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(2), // baz
                NodeIndex::new(3), // bar
                NodeIndex::new(1), // foo
                NodeIndex::new(5), // noConfig
                NodeIndex::new(4), // basic
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(0)],
                vec![NodeIndex::new(2), NodeIndex::new(3), NodeIndex::new(5)],
                vec![NodeIndex::new(1), NodeIndex::new(4)]
            ]
        );
    }

    #[tokio::test]
    async fn projects_with_tasks() {
        let projects = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph.sync_project("noConfig", &projects).unwrap();
        graph.sync_project("tasks", &projects).unwrap();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1), // noConfig
                NodeIndex::new(2), // tasks
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(0)],
                vec![NodeIndex::new(1), NodeIndex::new(2)]
            ]
        );
    }

    #[tokio::test]
    async fn avoids_dupe_projects() {
        let projects = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph.sync_project("advanced", &projects).unwrap();
        graph.sync_project("advanced", &projects).unwrap();
        graph.sync_project("advanced", &projects).unwrap();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    #[should_panic(expected = "Project(UnconfiguredID(\"unknown\"))")]
    async fn errors_for_unknown_project() {
        let projects = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph.sync_project("unknown", &projects).unwrap();

        assert_snapshot!(graph.to_dot());
    }
}
