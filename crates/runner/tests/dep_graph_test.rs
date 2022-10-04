use insta::assert_snapshot;
use moon_cache::CacheEngine;
use moon_config::{GlobalProjectConfig, WorkspaceConfig, WorkspaceProjects};
use moon_project_graph::ProjectGraph;
use moon_runner::{BatchedTopoSort, DepGraph, NodeIndex};
use moon_task::Target;
use moon_utils::test::{create_sandbox, TempDir};
use std::collections::{HashMap, HashSet};

async fn create_project_graph() -> (ProjectGraph, TempDir) {
    let fixture = create_sandbox("projects");
    let workspace_root = fixture.path();
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Map(HashMap::from([
            ("advanced".to_owned(), "advanced".to_owned()),
            ("basic".to_owned(), "basic".to_owned()),
            ("emptyConfig".to_owned(), "empty-config".to_owned()),
            ("noConfig".to_owned(), "no-config".to_owned()),
            ("foo".to_owned(), "deps/foo".to_owned()),
            ("bar".to_owned(), "deps/bar".to_owned()),
            ("baz".to_owned(), "deps/baz".to_owned()),
            ("tasks".to_owned(), "tasks".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    (
        ProjectGraph::create(
            workspace_root,
            &workspace_config,
            GlobalProjectConfig::default(),
            &CacheEngine::create(workspace_root).await.unwrap(),
        )
        .await
        .unwrap(),
        fixture,
    )
}

async fn create_tasks_project_graph() -> (ProjectGraph, TempDir) {
    let fixture = create_sandbox("tasks");
    let workspace_root = fixture.path();
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Map(HashMap::from([
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
        ])),
        ..WorkspaceConfig::default()
    };
    let global_config = GlobalProjectConfig {
        file_groups: HashMap::from([("sources".to_owned(), vec!["src/**/*".to_owned()])]),
        ..GlobalProjectConfig::default()
    };

    (
        ProjectGraph::create(
            workspace_root,
            &workspace_config,
            global_config,
            &CacheEngine::create(workspace_root).await.unwrap(),
        )
        .await
        .unwrap(),
        fixture,
    )
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
    let (projects, _fixture) = create_tasks_project_graph().await;

    let mut graph = DepGraph::default();
    graph
        .run_target(&Target::new("cycle", "a").unwrap(), &projects, &None)
        .unwrap();
    graph
        .run_target(&Target::new("cycle", "b").unwrap(), &projects, &None)
        .unwrap();
    graph
        .run_target(&Target::new("cycle", "c").unwrap(), &projects, &None)
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
        let (projects, _fixture) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("tasks", "test").unwrap(), &projects, &None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), &projects, &None)
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
        let (projects, _fixture) = create_tasks_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("basic", "test").unwrap(), &projects, &None)
            .unwrap();
        graph
            .run_target(&Target::new("basic", "lint").unwrap(), &projects, &None)
            .unwrap();
        graph
            .run_target(&Target::new("chain", "a").unwrap(), &projects, &None)
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
        let (projects, _fixture) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), &projects, &None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), &projects, &None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), &projects, &None)
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
        let (projects, _fixture) = create_tasks_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::parse(":build").unwrap(), &projects, &None)
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
        let (projects, _fixture) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::parse("^:lint").unwrap(), &projects, &None)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Target(NoProjectSelfInRunContext)")]
    async fn errors_for_target_self_scope() {
        let (projects, _fixture) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::parse("~:lint").unwrap(), &projects, &None)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Project(UnconfiguredID(\"unknown\"))")]
    async fn errors_for_unknown_project() {
        let (projects, _fixture) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("unknown", "test").unwrap(), &projects, &None)
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    #[should_panic(expected = "Project(UnconfiguredTask(\"build\", \"tasks\"))")]
    async fn errors_for_unknown_task() {
        let (projects, _fixture) = create_project_graph().await;

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("tasks", "build").unwrap(), &projects, &None)
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }
}

mod run_target_if_touched {
    use super::*;

    #[tokio::test]
    async fn skips_if_untouched_project() {
        let (projects, fixture) = create_tasks_project_graph().await;

        let mut touched_files = HashSet::new();
        touched_files.insert(fixture.path().join("input-a/a.ts"));
        touched_files.insert(fixture.path().join("input-c/c.ts"));
        let touched_files = Some(touched_files);

        let mut graph = DepGraph::default();
        graph
            .run_target(
                &Target::new("inputA", "a").unwrap(),
                &projects,
                &touched_files,
            )
            .unwrap();
        graph
            .run_target(
                &Target::new("inputB", "b").unwrap(),
                &projects,
                &touched_files,
            )
            .unwrap();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    async fn skips_if_untouched_task() {
        let (projects, fixture) = create_tasks_project_graph().await;

        let mut touched_files = HashSet::new();
        touched_files.insert(fixture.path().join("input-a/a2.ts"));
        touched_files.insert(fixture.path().join("input-b/b2.ts"));
        touched_files.insert(fixture.path().join("input-c/any.ts"));
        let touched_files = Some(touched_files);

        let mut graph = DepGraph::default();
        graph
            .run_target(
                &Target::new("inputA", "a").unwrap(),
                &projects,
                &touched_files,
            )
            .unwrap();
        graph
            .run_target(
                &Target::new("inputB", "b2").unwrap(),
                &projects,
                &touched_files,
            )
            .unwrap();
        graph
            .run_target(
                &Target::new("inputC", "c").unwrap(),
                &projects,
                &touched_files,
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
            let platform = graph.get_runtime_from_project(&project, projects);

            graph.sync_project(&platform, &project, projects).unwrap();
        }
    }

    #[tokio::test]
    async fn isolated_projects() {
        let (projects, _fixture) = create_project_graph().await;
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
        let (projects, _fixture) = create_project_graph().await;
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
        let (projects, _fixture) = create_project_graph().await;
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
        let (projects, _fixture) = create_project_graph().await;
        let mut graph = DepGraph::default();

        sync_projects(&mut graph, &projects, &["advanced", "advanced", "advanced"]);

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    #[should_panic(expected = "UnconfiguredID(\"unknown\")")]
    async fn errors_for_unknown_project() {
        let (projects, _fixture) = create_project_graph().await;
        let mut graph = DepGraph::default();

        sync_projects(&mut graph, &projects, &["unknown"]);

        assert_snapshot!(graph.to_dot());
    }
}
