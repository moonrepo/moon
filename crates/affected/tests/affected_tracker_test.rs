use moon_affected::*;
use moon_common::Id;
use moon_config::WorkspaceProjects;
use moon_env_var::GlobalEnvBag;
use moon_task::Target;
use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{Sandbox, create_sandbox};
use std::sync::Arc;

async fn build_graph_with_sandbox(fixture: &str) -> (WorkspaceGraph, Sandbox) {
    let sandbox = create_sandbox(fixture);

    (
        WorkspaceMocker::new(sandbox.path())
            .with_default_projects()
            .with_global_envs()
            .with_inherited_tasks()
            .mock_workspace_graph()
            .await,
        sandbox,
    )
}

async fn build_graph(fixture: &str) -> WorkspaceGraph {
    build_graph_with_sandbox(fixture).await.0
}

mod affected_projects {
    use super::*;

    // Dependents (reverse `dependsOn`) of the `projects` fixture:
    // d <- c <- b <- a <- f
    //      c <------/
    // e has no relationships

    fn create_state_from_file(file: &str) -> AffectedProjectState {
        let mut state = AffectedProjectState::default();
        state.files.insert(file.into());
        state
    }

    fn create_state_from_dependency(id: &str, distance: u32) -> AffectedProjectState {
        let mut state = AffectedProjectState::default();
        state.dependencies.insert(Id::raw(id));
        state.distance = distance;
        state
    }

    async fn track_with_depth(
        fixture: &str,
        files: &[&str],
        depth: Depth,
    ) -> FxHashMap<Id, AffectedProjectState> {
        let workspace_graph = build_graph(fixture).await;
        let changed_files = FxHashSet::from_iter(files.iter().map(|file| (*file).into()));

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker.set_depth(depth);
        tracker.track_projects().await.unwrap();
        tracker.build().projects
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn empty_if_no_changed_files() {
        let workspace_graph = build_graph("projects").await;
        let changed_files = FxHashSet::default();

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker.track_projects().await.unwrap();
        let affected = tracker.build();

        assert!(affected.projects.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tracks_projects() {
        let workspace_graph = build_graph("projects").await;
        let changed_files = FxHashSet::from_iter(["c/file.txt".into()]);

        // Deep by default
        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker.track_projects().await.unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.projects,
            FxHashMap::from_iter([
                // Reached directly from c, not through b
                (Id::raw("a"), create_state_from_dependency("c", 1)),
                (Id::raw("b"), create_state_from_dependency("c", 1)),
                (Id::raw("c"), create_state_from_file("c/file.txt")),
                (Id::raw("f"), create_state_from_dependency("a", 2)),
                (Id::raw("root"), create_state_from_file("c/file.txt")),
            ])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tracks_multiple_projects() {
        let affected = track_with_depth(
            "projects",
            &["a/file.txt", "b/file.txt", "e/file.txt"],
            Depth::NONE,
        )
        .await;

        assert_eq!(
            affected,
            FxHashMap::from_iter([
                (Id::raw("a"), create_state_from_file("a/file.txt")),
                (Id::raw("b"), create_state_from_file("b/file.txt")),
                (Id::raw("e"), create_state_from_file("e/file.txt")),
                (Id::raw("root"), create_state_from_file("a/file.txt")),
            ])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tracks_nested_projects() {
        let sandbox = create_sandbox("projects");
        sandbox.create_file("a/sub/file.txt", "");

        let workspace_graph = WorkspaceMocker::new(sandbox.path())
            .with_default_projects()
            .with_global_envs()
            .with_inherited_tasks()
            .update_workspace_config(|config| {
                if let WorkspaceProjects::Both(projects) = &mut config.projects {
                    projects.sources.insert(Id::raw("sub"), "a/sub".into());
                }
            })
            .mock_workspace_graph()
            .await;

        let changed_files = FxHashSet::from_iter(["a/sub/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker.set_depth(Depth::NONE);
        tracker.track_projects().await.unwrap();
        let affected = tracker.build();

        // Both the nested project, and the project it's nested within
        assert_eq!(
            affected.projects,
            FxHashMap::from_iter([
                (Id::raw("a"), create_state_from_file("a/sub/file.txt")),
                (Id::raw("sub"), create_state_from_file("a/sub/file.txt")),
                (Id::raw("root"), create_state_from_file("a/sub/file.txt")),
            ])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_track_root_for_hidden_files() {
        let workspace_graph = build_graph("projects").await;
        let changed_files = FxHashSet::from_iter([".github/workflows/ci.yml".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker.track_projects().await.unwrap();
        let affected = tracker.build();

        assert!(affected.projects.is_empty());
    }

    // Affectedness only flows from a project to its dependents,
    // never to the projects it depends on
    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_mark_dependencies() {
        let affected = track_with_depth("projects", &["a/file.txt"], Depth::Deep).await;

        assert_eq!(
            affected,
            FxHashMap::from_iter([
                (Id::raw("a"), create_state_from_file("a/file.txt")),
                (Id::raw("f"), create_state_from_dependency("a", 1)),
                (Id::raw("root"), create_state_from_file("a/file.txt")),
            ])
        );
    }

    mod checked {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn marked_when_tracked() {
            let workspace_graph = Arc::new(build_graph("projects").await);
            let changed_files = FxHashSet::from_iter(["d/file.txt".into()]);

            let mut tracker = AffectedTracker::new(Arc::clone(&workspace_graph), changed_files);
            tracker.track_projects().await.unwrap();

            // Directly affected
            let d = workspace_graph.get_project("d").unwrap();

            assert_eq!(
                tracker.is_project_affected(&d),
                Some(AffectedBy::AlreadyMarked)
            );
            assert!(tracker.is_project_marked(&d));
            assert!(tracker.is_project_marked_ignoring_relations(&d));

            // Marked through a relationship only, so not directly affected
            let c = workspace_graph.get_project("c").unwrap();

            assert_eq!(tracker.is_project_affected(&c), None);
            assert!(tracker.is_project_marked(&c));
            assert!(!tracker.is_project_marked_ignoring_relations(&c));

            // Not affected at all
            let e = workspace_graph.get_project("e").unwrap();

            assert_eq!(tracker.is_project_affected(&e), None);
            assert!(!tracker.is_project_marked(&e));
        }
    }

    mod project_dependents {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn none() {
            let affected = track_with_depth("projects", &["d/file.txt"], Depth::NONE).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (Id::raw("d"), create_state_from_file("d/file.txt")),
                    (Id::raw("root"), create_state_from_file("d/file.txt")),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn direct() {
            let affected = track_with_depth("projects", &["d/file.txt"], Depth::DIRECT).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (Id::raw("c"), create_state_from_dependency("d", 1)),
                    (Id::raw("d"), create_state_from_file("d/file.txt")),
                    (Id::raw("root"), create_state_from_file("d/file.txt")),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn direct_no_dependents() {
            let affected = track_with_depth("projects", &["e/file.txt"], Depth::DIRECT).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (Id::raw("e"), create_state_from_file("e/file.txt")),
                    (Id::raw("root"), create_state_from_file("e/file.txt")),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn limit() {
            let affected = track_with_depth("projects", &["d/file.txt"], Depth::Limit(2)).await;

            // f is 3 hops away
            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (Id::raw("a"), create_state_from_dependency("c", 2)),
                    (Id::raw("b"), create_state_from_dependency("c", 2)),
                    (Id::raw("c"), create_state_from_dependency("d", 1)),
                    (Id::raw("d"), create_state_from_file("d/file.txt")),
                    (Id::raw("root"), create_state_from_file("d/file.txt")),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn deep() {
            let affected = track_with_depth("projects", &["d/file.txt"], Depth::Deep).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (Id::raw("a"), create_state_from_dependency("c", 2)),
                    (Id::raw("b"), create_state_from_dependency("c", 2)),
                    (Id::raw("c"), create_state_from_dependency("d", 1)),
                    (Id::raw("d"), create_state_from_file("d/file.txt")),
                    (Id::raw("f"), create_state_from_dependency("a", 3)),
                    (Id::raw("root"), create_state_from_file("d/file.txt")),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn deep_no_dependents() {
            let affected = track_with_depth("projects", &["e/file.txt"], Depth::Deep).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (Id::raw("e"), create_state_from_file("e/file.txt")),
                    (Id::raw("root"), create_state_from_file("e/file.txt")),
                ])
            );
        }

        // The cycle crosses the production and development partitions,
        // which the graph allows, so the walk must still terminate
        #[tokio::test(flavor = "multi_thread")]
        async fn deep_cycle() {
            let affected =
                track_with_depth("projects-cycle", &["cycle-c/file.txt"], Depth::Deep).await;

            // The changed project is never marked through the cycle
            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (
                        Id::raw("cycle-a"),
                        create_state_from_dependency("cycle-b", 2)
                    ),
                    (
                        Id::raw("cycle-b"),
                        create_state_from_dependency("cycle-c", 1)
                    ),
                    (
                        Id::raw("cycle-c"),
                        create_state_from_file("cycle-c/file.txt")
                    ),
                    (Id::raw("root"), create_state_from_file("cycle-c/file.txt")),
                ])
            );
        }

        // a is 2 hops from d (through c), but only 1 hop from b, so it's
        // within a direct depth, and only marked through b
        #[tokio::test(flavor = "multi_thread")]
        async fn minimum_distance_from_multiple_projects() {
            let affected =
                track_with_depth("projects", &["b/file.txt", "d/file.txt"], Depth::DIRECT).await;

            // Root is affected by any file, so isn't asserted
            assert_eq!(affected.len(), 5);
            assert_eq!(
                affected[&Id::raw("a")],
                create_state_from_dependency("b", 1)
            );
            assert_eq!(
                affected[&Id::raw("b")],
                create_state_from_file("b/file.txt")
            );
            assert_eq!(
                affected[&Id::raw("c")],
                create_state_from_dependency("d", 1)
            );
            assert_eq!(
                affected[&Id::raw("d")],
                create_state_from_file("d/file.txt")
            );
            assert!(!affected.contains_key(&Id::raw("f")));
        }

        // A project that's directly affected may also be reached through
        // a dependency, but it's only walked from once
        #[tokio::test(flavor = "multi_thread")]
        async fn changed_dependents_keep_their_own_reason() {
            let affected =
                track_with_depth("projects", &["c/file.txt", "d/file.txt"], Depth::Deep).await;

            assert_eq!(
                affected[&Id::raw("c")],
                create_state_from_file("c/file.txt")
            );
            assert_eq!(
                affected[&Id::raw("b")],
                create_state_from_dependency("c", 1)
            );
            assert_eq!(
                affected[&Id::raw("a")],
                create_state_from_dependency("c", 1)
            );
            assert_eq!(
                affected[&Id::raw("f")],
                create_state_from_dependency("a", 2)
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn lazily_marked_project_walks_dependents() {
            let workspace_graph = Arc::new(build_graph("projects").await);
            let d = workspace_graph.get_project("d").unwrap();

            let mut tracker =
                AffectedTracker::new(Arc::clone(&workspace_graph), FxHashSet::default());
            tracker.set_depth(Depth::DIRECT);
            tracker
                .mark_project_affected(&d, AffectedBy::AlwaysAffected)
                .unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (
                        Id::raw("d"),
                        AffectedProjectState {
                            other: true,
                            ..Default::default()
                        }
                    ),
                    (Id::raw("c"), create_state_from_dependency("d", 1)),
                ])
            );
        }
    }
}

mod affected_tasks {
    use super::*;

    fn create_state_from_file(file: &str) -> AffectedTaskState {
        let mut state = AffectedTaskState::default();
        state.files.insert(file.into());
        state
    }

    fn create_state_from_env(env: &str) -> AffectedTaskState {
        let mut state = AffectedTaskState::default();
        state.env.insert(env.into());
        state
    }

    fn create_state_from_dependency(id: &str, distance: u32) -> AffectedTaskState {
        let mut state = AffectedTaskState::default();
        state.dependencies.insert(Target::parse(id).unwrap());
        state.distance = distance;
        state
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn empty_if_no_changed_files() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::default();

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker.track_tasks().await.unwrap();
        let affected = tracker.build();

        assert!(affected.tasks.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn not_affected_if_no_inputs() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:no-inputs").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        assert!(affected.tasks.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn affected_by_file() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-file").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([(
                Target::parse("base:by-file").unwrap(),
                create_state_from_file("base/file.txt")
            )])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn affected_by_file_if_content_matches() {
        let (workspace_graph, sandbox) = build_graph_with_sandbox("tasks").await;
        sandbox.create_file("base/file.txt", "bar");

        let changed_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-file-match").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([(
                Target::parse("base:by-file-match").unwrap(),
                create_state_from_file("base/file.txt")
            )])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn not_affected_by_file_if_content_doesnt_match() {
        let (workspace_graph, sandbox) = build_graph_with_sandbox("tasks").await;
        sandbox.create_file("base/file.txt", "foo");

        let changed_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-file-match").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        assert!(affected.tasks.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn not_affected_by_file_if_content_matched_file_is_missing() {
        // The file was changed (deleted), but doesn't exist to match against
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-file-match").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        assert!(affected.tasks.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn not_affected_by_file_if_not_an_input() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["base/other.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-file").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        assert!(affected.tasks.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn affected_by_glob() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-glob").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([(
                Target::parse("base:by-glob").unwrap(),
                create_state_from_file("base/file.txt")
            )])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn affected_by_env_var() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::default();
        let bag = GlobalEnvBag::instance();

        bag.set("ENV", "affected");

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-env").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([(
                Target::parse("base:by-env").unwrap(),
                create_state_from_env("ENV")
            )])
        );

        bag.remove("ENV");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn self_scope() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["self/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("self:b").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        // a depends on `~:b`, c depends on `b`, and parent:child depends
        // on `self:b`, so all are 1 hop away
        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([
                (
                    Target::parse("parent:child").unwrap(),
                    create_state_from_dependency("self:b", 1)
                ),
                (
                    Target::parse("self:a").unwrap(),
                    create_state_from_dependency("self:b", 1)
                ),
                (
                    Target::parse("self:b").unwrap(),
                    create_state_from_file("self/file.txt")
                ),
                (
                    Target::parse("self:c").unwrap(),
                    create_state_from_dependency("self:b", 1)
                ),
            ])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parent_scope() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:a").unwrap()])
            .await
            .unwrap();
        let affected = tracker.build();

        // parent:child depends on `^:a`
        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([
                (
                    Target::parse("base:a").unwrap(),
                    create_state_from_file("base/file.txt")
                ),
                (
                    Target::parse("parent:child").unwrap(),
                    create_state_from_dependency("base:a", 1)
                ),
            ])
        );
    }

    // Affectedness only flows from a task to its dependents,
    // never to the tasks it depends on
    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_mark_dependency_if_dependent_is_changed() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["downstream/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker.set_depth(Depth::Deep);
        tracker.track_tasks().await.unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([
                (
                    Target::parse("downstream:global").unwrap(),
                    create_state_from_file("downstream/file.txt")
                ),
                (
                    Target::parse("downstream:task").unwrap(),
                    create_state_from_file("downstream/file.txt")
                ),
            ])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn marks_dependent_if_dependency_is_changed() {
        let workspace_graph = build_graph("tasks").await;
        let changed_files = FxHashSet::from_iter(["upstream/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
        tracker.set_depth(Depth::DIRECT);
        tracker.track_tasks().await.unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([
                (
                    Target::parse("upstream:task").unwrap(),
                    create_state_from_file("upstream/file.txt")
                ),
                (
                    Target::parse("upstream:global").unwrap(),
                    create_state_from_file("upstream/file.txt")
                ),
                (
                    Target::parse("downstream:task").unwrap(),
                    create_state_from_dependency("upstream:task", 1)
                ),
            ])
        );
    }

    mod checked {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_check_tracked_tasks_again() {
            let workspace_graph = Arc::new(build_graph("tasks").await);
            let target = Target::parse("base:by-env").unwrap();
            let task = workspace_graph.get_task(&target).unwrap();
            let bag = GlobalEnvBag::instance();

            bag.remove("ENV");

            let mut tracker =
                AffectedTracker::new(Arc::clone(&workspace_graph), FxHashSet::default());
            tracker.track_tasks_by_target(&[target]).await.unwrap();

            // Would now be affected if it was checked again
            bag.set("ENV", "affected");

            assert_eq!(tracker.is_task_affected(&task).unwrap(), None);
            assert!(!tracker.is_task_marked(&task));

            bag.remove("ENV");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn checks_untracked_tasks() {
            let workspace_graph = Arc::new(build_graph("tasks").await);
            let task = workspace_graph
                .get_task(&Target::parse("base:by-env").unwrap())
                .unwrap();
            let bag = GlobalEnvBag::instance();

            bag.remove("ENV");

            let mut tracker =
                AffectedTracker::new(Arc::clone(&workspace_graph), FxHashSet::default());
            tracker
                .track_tasks_by_target(&[Target::parse("base:by-file").unwrap()])
                .await
                .unwrap();

            bag.set("ENV", "affected");

            assert_eq!(
                tracker.is_task_affected(&task).unwrap(),
                Some(AffectedBy::EnvironmentVariable("ENV".into()))
            );

            bag.remove("ENV");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_checks() {
            let workspace_graph = Arc::new(build_graph("tasks").await);
            let task = workspace_graph
                .get_task(&Target::parse("base:by-env").unwrap())
                .unwrap();
            let bag = GlobalEnvBag::instance();

            bag.remove("ENV");

            let mut tracker =
                AffectedTracker::new(Arc::clone(&workspace_graph), FxHashSet::default());

            assert_eq!(tracker.is_task_affected(&task).unwrap(), None);

            // Would now be affected if it was checked again
            bag.set("ENV", "affected");

            assert_eq!(tracker.is_task_affected(&task).unwrap(), None);

            bag.remove("ENV");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn marked_when_tracked() {
            let workspace_graph = Arc::new(build_graph("tasks").await);
            let changed_files = FxHashSet::from_iter(["chain/c.txt".into()]);

            let mut tracker = AffectedTracker::new(Arc::clone(&workspace_graph), changed_files);
            tracker.track_tasks().await.unwrap();

            // Directly affected
            let c = workspace_graph
                .get_task(&Target::parse("chain:c").unwrap())
                .unwrap();

            assert_eq!(
                tracker.is_task_affected(&c).unwrap(),
                Some(AffectedBy::AlreadyMarked)
            );
            assert!(tracker.is_task_marked(&c));
            assert!(tracker.is_task_marked_ignoring_relations(&c));

            // Marked through a relationship only, so not directly affected
            let b = workspace_graph
                .get_task(&Target::parse("chain:b").unwrap())
                .unwrap();

            assert_eq!(tracker.is_task_affected(&b).unwrap(), None);
            assert!(tracker.is_task_marked(&b));
            assert!(!tracker.is_task_marked_ignoring_relations(&b));

            // Not affected at all
            let z = workspace_graph
                .get_task(&Target::parse("chain:z").unwrap())
                .unwrap();

            assert_eq!(tracker.is_task_affected(&z).unwrap(), None);
            assert!(!tracker.is_task_marked(&z));
        }
    }

    mod task_dependents {
        use super::*;

        // Dependents (reverse `deps`) of the `chain` fixture:
        // e <- d <- c <- b <- a
        // z has no relationships

        async fn track_with_depth(
            files: &[&str],
            depth: Depth,
        ) -> FxHashMap<Target, AffectedTaskState> {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(files.iter().map(|file| (*file).into()));

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_depth(depth);
            tracker.track_tasks().await.unwrap();
            tracker.build().tasks
        }

        fn target(id: &str) -> Target {
            Target::parse(id).unwrap()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn none() {
            let affected = track_with_depth(&["chain/e.txt"], Depth::NONE).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (
                        target("chain:global"),
                        create_state_from_file("chain/e.txt")
                    ),
                    (target("chain:e"), create_state_from_file("chain/e.txt")),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn direct() {
            let affected = track_with_depth(&["chain/e.txt"], Depth::DIRECT).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (
                        target("chain:global"),
                        create_state_from_file("chain/e.txt")
                    ),
                    (target("chain:e"), create_state_from_file("chain/e.txt")),
                    (
                        target("chain:d"),
                        create_state_from_dependency("chain:e", 1)
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn direct_no_dependents() {
            let affected = track_with_depth(&["chain/z.txt"], Depth::DIRECT).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (
                        target("chain:global"),
                        create_state_from_file("chain/z.txt")
                    ),
                    (target("chain:z"), create_state_from_file("chain/z.txt")),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn limit() {
            let affected = track_with_depth(&["chain/e.txt"], Depth::Limit(2)).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (
                        target("chain:global"),
                        create_state_from_file("chain/e.txt")
                    ),
                    (target("chain:e"), create_state_from_file("chain/e.txt")),
                    (
                        target("chain:d"),
                        create_state_from_dependency("chain:e", 1)
                    ),
                    (
                        target("chain:c"),
                        create_state_from_dependency("chain:d", 2)
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn deep() {
            let affected = track_with_depth(&["chain/e.txt"], Depth::Deep).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (
                        target("chain:global"),
                        create_state_from_file("chain/e.txt")
                    ),
                    (target("chain:e"), create_state_from_file("chain/e.txt")),
                    (
                        target("chain:d"),
                        create_state_from_dependency("chain:e", 1)
                    ),
                    (
                        target("chain:c"),
                        create_state_from_dependency("chain:d", 2)
                    ),
                    (
                        target("chain:b"),
                        create_state_from_dependency("chain:c", 3)
                    ),
                    (
                        target("chain:a"),
                        create_state_from_dependency("chain:b", 4)
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn deep_no_dependents() {
            let affected = track_with_depth(&["chain/z.txt"], Depth::Deep).await;

            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (
                        target("chain:global"),
                        create_state_from_file("chain/z.txt")
                    ),
                    (target("chain:z"), create_state_from_file("chain/z.txt")),
                ])
            );
        }

        // A task that's directly affected may also be reached through a
        // dependency, but keeps only its own reason, and the walk continues
        // through it from its own distance
        #[tokio::test(flavor = "multi_thread")]
        async fn deep_from_multiple_tasks() {
            let affected = track_with_depth(&["chain/c.txt", "chain/e.txt"], Depth::Deep).await;

            assert_eq!(
                affected[&target("chain:e")],
                create_state_from_file("chain/e.txt")
            );
            assert_eq!(
                affected[&target("chain:d")],
                create_state_from_dependency("chain:e", 1)
            );
            assert_eq!(
                affected[&target("chain:c")],
                create_state_from_file("chain/c.txt")
            );
            assert_eq!(
                affected[&target("chain:b")],
                create_state_from_dependency("chain:c", 1)
            );
            assert_eq!(
                affected[&target("chain:a")],
                create_state_from_dependency("chain:b", 2)
            );
            assert!(!affected.contains_key(&target("chain:z")));
        }

        // Dependents (reverse `deps`) of the `depth` fixture:
        // base <- mid1 <- mid2 <- top <- above
        //                other <-/

        #[tokio::test(flavor = "multi_thread")]
        async fn limit_uses_minimum_distance_from_multiple_tasks() {
            let affected =
                track_with_depth(&["depth/base.txt", "depth/other.txt"], Depth::Limit(2)).await;

            // top is 3 hops from base, but 1 hop from other,
            // so it and its own dependent are within range
            assert_eq!(
                affected,
                FxHashMap::from_iter([
                    (target("depth:global"), {
                        let mut state = AffectedTaskState::default();
                        state
                            .files
                            .extend(affected[&target("depth:global")].files.clone());
                        state
                    }),
                    (
                        target("depth:base"),
                        create_state_from_file("depth/base.txt")
                    ),
                    (
                        target("depth:other"),
                        create_state_from_file("depth/other.txt")
                    ),
                    (
                        target("depth:mid1"),
                        create_state_from_dependency("depth:base", 1)
                    ),
                    (
                        target("depth:mid2"),
                        create_state_from_dependency("depth:mid1", 2)
                    ),
                    (
                        target("depth:top"),
                        create_state_from_dependency("depth:other", 1)
                    ),
                    (
                        target("depth:above"),
                        create_state_from_dependency("depth:top", 2)
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn records_only_the_closest_dependency() {
            let affected =
                track_with_depth(&["depth/base.txt", "depth/other.txt"], Depth::Deep).await;

            // top depends on mid2 (affected through base) and other, but
            // is visited once, at its minimum distance, through other
            assert_eq!(
                affected[&target("depth:top")],
                create_state_from_dependency("depth:other", 1)
            );
            assert_eq!(
                affected[&target("depth:above")],
                create_state_from_dependency("depth:top", 2)
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn single_seed_beyond_limit() {
            let affected = track_with_depth(&["depth/base.txt"], Depth::Limit(3)).await;

            assert_eq!(
                affected[&target("depth:top")],
                create_state_from_dependency("depth:mid2", 3)
            );
            assert!(!affected.contains_key(&target("depth:above")));
            assert!(!affected.contains_key(&target("depth:other")));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn lazily_marked_task_walks_dependents() {
            let workspace_graph = Arc::new(build_graph("tasks").await);
            let e = workspace_graph.get_task(&target("chain:e")).unwrap();

            let mut tracker =
                AffectedTracker::new(Arc::clone(&workspace_graph), FxHashSet::default());
            tracker.set_depth(Depth::Limit(2));
            tracker
                .mark_task_affected(&e, AffectedBy::AlwaysAffected)
                .unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        target("chain:e"),
                        AffectedTaskState {
                            other: true,
                            ..Default::default()
                        }
                    ),
                    (
                        target("chain:d"),
                        create_state_from_dependency("chain:e", 1)
                    ),
                    (
                        target("chain:c"),
                        create_state_from_dependency("chain:d", 2)
                    ),
                ])
            );

            // The owning project is affected by the task
            assert_eq!(
                affected.projects[&Id::raw("chain")].tasks,
                FxHashSet::from_iter([target("chain:e")])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn lazily_already_marked_task_walks_dependents() {
            let workspace_graph = Arc::new(build_graph("tasks").await);
            let d = workspace_graph.get_task(&target("chain:d")).unwrap();

            let mut tracker =
                AffectedTracker::new(Arc::clone(&workspace_graph), FxHashSet::default());
            tracker.set_depth(Depth::DIRECT);
            tracker
                .mark_task_affected(&d, AffectedBy::AlreadyMarked)
                .unwrap();
            let affected = tracker.build();

            // Not marked itself, but its dependents are
            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([(
                    target("chain:c"),
                    create_state_from_dependency("chain:d", 1)
                )])
            );
        }

        // Each walk visits a task at its minimum distance from the tasks it
        // started from, but separate walks may reach it from different
        // distances, so the closest one is reported
        #[tokio::test(flavor = "multi_thread")]
        async fn minimum_distance_over_separate_walks() {
            let workspace_graph = Arc::new(build_graph("tasks").await);
            let e = workspace_graph.get_task(&target("chain:e")).unwrap();
            let c = workspace_graph.get_task(&target("chain:c")).unwrap();

            let mut tracker =
                AffectedTracker::new(Arc::clone(&workspace_graph), FxHashSet::default());
            tracker.set_depth(Depth::Deep);
            tracker
                .mark_task_affected(&e, AffectedBy::AlwaysAffected)
                .unwrap();
            tracker
                .mark_task_affected(&c, AffectedBy::AlwaysAffected)
                .unwrap();
            let affected = tracker.build();

            // Directly affected, even though also reached from e
            assert_eq!(affected.tasks[&target("chain:c")].distance, 0);
            assert!(
                affected.tasks[&target("chain:c")]
                    .dependencies
                    .contains(&target("chain:d"))
            );

            // 3 hops from e, but 1 hop from c
            assert_eq!(
                affected.tasks[&target("chain:b")],
                create_state_from_dependency("chain:c", 1)
            );
            assert_eq!(
                affected.tasks[&target("chain:a")],
                create_state_from_dependency("chain:b", 2)
            );
            assert_eq!(affected.tasks[&target("chain:d")].distance, 1);
        }

        #[test]
        fn state_distance_is_zero_when_directly_affected() {
            let dependency = || AffectedBy::DependencyTask {
                target: target("chain:d"),
                distance: 2,
            };

            assert_eq!(
                AffectedTaskState::from(FxHashSet::from_iter([
                    dependency(),
                    AffectedBy::DependencyTask {
                        target: target("chain:z"),
                        distance: 1,
                    },
                ]))
                .distance,
                1
            );

            for direct in [
                AffectedBy::AlwaysAffected,
                AffectedBy::ChangedFile("chain/c.txt".into()),
                AffectedBy::EnvironmentVariable("ENV".into()),
            ] {
                assert_eq!(
                    AffectedTaskState::from(FxHashSet::from_iter([direct, dependency(),])).distance,
                    0
                );
            }
        }
    }

    mod project_sources {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn any_file() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["dep/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.track_tasks().await.unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("project-sources:by-any").unwrap(),
                        create_state_from_file("dep/file.txt")
                    ),
                    (
                        Target::parse("dep:global").unwrap(),
                        create_state_from_file("dep/file.txt")
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn using_file_group() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["dep/src/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.track_tasks().await.unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("project-sources:by-group").unwrap(),
                        create_state_from_file("dep/src/file.txt")
                    ),
                    (
                        Target::parse("project-sources:by-any").unwrap(),
                        create_state_from_file("dep/src/file.txt")
                    ),
                    (
                        Target::parse("dep:global").unwrap(),
                        create_state_from_file("dep/src/file.txt")
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn using_filter_glob() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["dep/tests/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.track_tasks().await.unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("project-sources:by-filter").unwrap(),
                        create_state_from_file("dep/tests/file.txt")
                    ),
                    (
                        Target::parse("project-sources:by-any").unwrap(),
                        create_state_from_file("dep/tests/file.txt")
                    ),
                    (
                        Target::parse("project-sources:by-deps").unwrap(),
                        create_state_from_file("dep/tests/file.txt")
                    ),
                    (
                        Target::parse("dep:global").unwrap(),
                        create_state_from_file("dep/tests/file.txt")
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn using_project_deps() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["dep/tests/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.track_tasks().await.unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("project-sources:by-filter").unwrap(),
                        create_state_from_file("dep/tests/file.txt")
                    ),
                    (
                        Target::parse("project-sources:by-any").unwrap(),
                        create_state_from_file("dep/tests/file.txt")
                    ),
                    (
                        Target::parse("project-sources:by-deps").unwrap(),
                        create_state_from_file("dep/tests/file.txt")
                    ),
                    (
                        Target::parse("dep:global").unwrap(),
                        create_state_from_file("dep/tests/file.txt")
                    ),
                ])
            );
        }
    }

    mod ci {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn when_ci_tracks_for_true() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(true);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:enabled").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([(
                    Target::parse("ci:enabled").unwrap(),
                    create_state_from_file("ci/file.txt")
                )])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn when_not_ci_tracks_for_true() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(false);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:enabled").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([(
                    Target::parse("ci:enabled").unwrap(),
                    create_state_from_file("ci/file.txt")
                )])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn when_ci_doesnt_track_for_false() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(true);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:disabled").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert!(affected.tasks.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn when_not_ci_tracks_for_false() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(false);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:disabled").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([(
                    Target::parse("ci:disabled").unwrap(),
                    create_state_from_file("ci/file.txt")
                )])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn when_ci_always_tracks_if_not_changed() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::default();

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(true);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:always").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([(
                    Target::parse("ci:always").unwrap(),
                    AffectedTaskState {
                        other: true,
                        ..Default::default()
                    }
                )])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn when_ci_only_tracks_when_ci() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(true);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:only").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([(
                    Target::parse("ci:only").unwrap(),
                    create_state_from_file("ci/file.txt")
                )])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn when_ci_only_doesnt_track_when_not_in_ci() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(false);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:only").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert!(affected.tasks.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn when_ci_doesnt_track_for_skip() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(true);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:skip").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert!(affected.tasks.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn when_not_ci_tracks_for_skip() {
            let workspace_graph = build_graph("tasks").await;
            let changed_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), changed_files);
            tracker.set_ci_check(false);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:skip").unwrap()])
                .await
                .unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([(
                    Target::parse("ci:skip").unwrap(),
                    create_state_from_file("ci/file.txt")
                )])
            );
        }
    }
}

mod depth {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Args {
        #[arg(long, value_parser = clap::value_parser!(Depth), default_value_t = Depth::Deep)]
        depth: Depth,
    }

    #[test]
    fn parses_with_clap() {
        let parse =
            |args: &[&str]| Args::try_parse_from([&["test"], args].concat()).map(|args| args.depth);

        assert_eq!(parse(&[]).unwrap(), Depth::Deep);
        assert_eq!(parse(&["--depth", "none"]).unwrap(), Depth::NONE);
        assert_eq!(parse(&["--depth", "Direct"]).unwrap(), Depth::DIRECT);
        assert_eq!(parse(&["--depth", "3"]).unwrap(), Depth::Limit(3));
        assert_eq!(parse(&["--depth=deep"]).unwrap(), Depth::Deep);
        assert!(parse(&["--depth", "-1"]).is_err());
        assert!(parse(&["--depth", "wide"]).is_err());
    }
}
