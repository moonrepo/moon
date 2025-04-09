use moon_affected::*;
use moon_common::Id;
use moon_env_var::GlobalEnvBag;
use moon_task::Target;
use moon_test_utils2::{WorkspaceGraph, WorkspaceMocker};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::create_sandbox;

async fn build_graph(fixture: &str) -> WorkspaceGraph {
    let sandbox = create_sandbox(fixture);

    WorkspaceMocker::new(sandbox.path())
        .with_default_projects()
        .with_global_envs()
        .with_inherited_tasks()
        .mock_workspace_graph()
        .await
}

mod affected_projects {
    use super::*;

    fn create_state_from_file(file: &str) -> AffectedProjectState {
        let mut state = AffectedProjectState::default();
        state.files.insert(file.into());
        state
    }

    fn create_state_from_dependency(id: &str) -> AffectedProjectState {
        let mut state = AffectedProjectState::default();
        state.upstream.insert(Id::raw(id));
        state
    }

    fn create_state_from_dependencies(ids: &[&str]) -> AffectedProjectState {
        let mut state = AffectedProjectState::default();
        state.upstream.extend(ids.iter().map(Id::raw));
        state
    }

    fn create_state_from_dependent(id: &str) -> AffectedProjectState {
        let mut state = AffectedProjectState::default();
        state.downstream.insert(Id::raw(id));
        state
    }

    fn create_state_from_dependents(ids: &[&str]) -> AffectedProjectState {
        let mut state = AffectedProjectState::default();
        state.downstream.extend(ids.iter().map(Id::raw));
        state
    }

    #[tokio::test]
    async fn empty_if_no_touched_files() {
        let workspace_graph = build_graph("projects").await;
        let touched_files = FxHashSet::default();

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker.track_projects().unwrap();
        let affected = tracker.build();

        assert!(affected.projects.is_empty());
    }

    #[tokio::test]
    async fn tracks_projects() {
        let workspace_graph = build_graph("projects").await;
        let touched_files = FxHashSet::from_iter(["a/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker.track_projects().unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.projects,
            FxHashMap::from_iter([
                (Id::raw("a"), create_state_from_file("a/file.txt")),
                (Id::raw("b"), create_state_from_dependent("a")),
                (Id::raw("c"), create_state_from_dependents(&["a", "b"])),
                (Id::raw("d"), create_state_from_dependent("c")),
                (Id::raw("root"), create_state_from_file("a/file.txt")),
            ])
        );
    }

    #[tokio::test]
    async fn tracks_multiple_projects() {
        let workspace_graph = build_graph("projects").await;
        let touched_files = FxHashSet::from_iter([
            "a/file.txt".into(),
            "b/file.txt".into(),
            "e/file.txt".into(),
        ]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::None);
        tracker.track_projects().unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.projects,
            FxHashMap::from_iter([
                (Id::raw("a"), create_state_from_file("a/file.txt")),
                (Id::raw("b"), create_state_from_file("b/file.txt")),
                (Id::raw("e"), create_state_from_file("e/file.txt")),
                (Id::raw("root"), create_state_from_file("a/file.txt")),
            ])
        );
    }

    mod project_upstream {
        use super::*;

        #[tokio::test]
        async fn none() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["a/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::None);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("a"), create_state_from_file("a/file.txt")),
                    (Id::raw("root"), create_state_from_file("a/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn direct() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["a/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::Direct, DownstreamScope::None);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("a"), create_state_from_file("a/file.txt")),
                    (Id::raw("b"), create_state_from_dependent("a")),
                    (Id::raw("c"), create_state_from_dependent("a")),
                    (Id::raw("root"), create_state_from_file("a/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn direct_no_deps() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["e/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::Direct, DownstreamScope::None);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("e"), create_state_from_file("e/file.txt")),
                    (Id::raw("root"), create_state_from_file("e/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn deep() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["a/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::Deep, DownstreamScope::None);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("a"), create_state_from_file("a/file.txt")),
                    (Id::raw("b"), create_state_from_dependent("a")),
                    (Id::raw("c"), create_state_from_dependents(&["a", "b"])),
                    (Id::raw("d"), create_state_from_dependent("c")),
                    (Id::raw("root"), create_state_from_file("a/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn deep_no_deps() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["e/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::Deep, DownstreamScope::None);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("e"), create_state_from_file("e/file.txt")),
                    (Id::raw("root"), create_state_from_file("e/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn deep_cycle() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["cycle-a/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::Deep, DownstreamScope::None);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("cycle-a"), {
                        let mut state = create_state_from_file("cycle-a/file.txt");
                        state.downstream.insert(Id::raw("cycle-c"));
                        state
                    }),
                    (Id::raw("cycle-b"), create_state_from_dependent("cycle-a")),
                    (Id::raw("cycle-c"), create_state_from_dependent("cycle-b")),
                    (Id::raw("root"), create_state_from_file("cycle-a/file.txt")),
                ])
            );
        }
    }

    mod project_downstream {
        use super::*;

        #[tokio::test]
        async fn none() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["c/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::None);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("c"), create_state_from_file("c/file.txt")),
                    (Id::raw("root"), create_state_from_file("c/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn direct() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["c/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::Direct);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("a"), create_state_from_dependency("c")),
                    (Id::raw("b"), create_state_from_dependency("c")),
                    (Id::raw("c"), create_state_from_file("c/file.txt")),
                    (Id::raw("root"), create_state_from_file("c/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn direct_no_deps() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["e/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::Direct);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("e"), create_state_from_file("e/file.txt")),
                    (Id::raw("root"), create_state_from_file("e/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn deep() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["c/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::Deep);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("a"), create_state_from_dependencies(&["b", "c"])),
                    (Id::raw("b"), create_state_from_dependency("c")),
                    (Id::raw("c"), create_state_from_file("c/file.txt")),
                    (Id::raw("root"), create_state_from_file("c/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn deep_no_deps() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["e/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::Deep);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("e"), create_state_from_file("e/file.txt")),
                    (Id::raw("root"), create_state_from_file("e/file.txt")),
                ])
            );
        }

        #[tokio::test]
        async fn deep_cycle() {
            let workspace_graph = build_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["cycle-c/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::Deep);
            tracker.track_projects().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.projects,
                FxHashMap::from_iter([
                    (Id::raw("cycle-b"), create_state_from_dependency("cycle-c")),
                    // (Id::raw("cycle-a"), create_state_from_dependency("cycle-b")),
                    (
                        Id::raw("cycle-c"),
                        create_state_from_file("cycle-c/file.txt")
                    ),
                    (Id::raw("root"), create_state_from_file("cycle-c/file.txt")),
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

    fn create_state_from_dependency(id: &str) -> AffectedTaskState {
        let mut state = AffectedTaskState::default();
        state.upstream.insert(Target::parse(id).unwrap());
        state
    }

    // fn create_state_from_dependencies(ids: &[&str]) -> AffectedTaskState {
    //     let mut state = AffectedTaskState::default();
    //     state
    //         .upstream
    //         .extend(ids.iter().map(|id| Target::parse(id).unwrap()));
    //     state
    // }

    fn create_state_from_dependent(id: &str) -> AffectedTaskState {
        let mut state = AffectedTaskState::default();
        state.downstream.insert(Target::parse(id).unwrap());
        state
    }

    fn create_state_from_dependents(ids: &[&str]) -> AffectedTaskState {
        let mut state = AffectedTaskState::default();
        state
            .downstream
            .extend(ids.iter().map(|id| Target::parse(id).unwrap()));
        state
    }

    #[tokio::test]
    async fn empty_if_no_touched_files() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::default();

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker.track_tasks().unwrap();
        let affected = tracker.build();

        assert!(affected.tasks.is_empty());
    }

    #[tokio::test]
    async fn not_affected_if_no_inputs() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:no-inputs").unwrap()])
            .unwrap();
        let affected = tracker.build();

        assert!(affected.tasks.is_empty());
    }

    #[tokio::test]
    async fn affected_by_file() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-file").unwrap()])
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

    #[tokio::test]
    async fn affected_by_glob() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["base/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-glob").unwrap()])
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

    #[tokio::test]
    async fn affected_by_env_var() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::default();
        let bag = GlobalEnvBag::instance();

        bag.set("ENV", "affected");

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker
            .track_tasks_by_target(&[Target::parse("base:by-env").unwrap()])
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

    #[tokio::test]
    async fn self_scope() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["self/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker
            .track_tasks_by_target(&[Target::parse("self:c").unwrap()])
            .unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([
                (
                    Target::parse("self:c").unwrap(),
                    create_state_from_file("self/file.txt")
                ),
                (
                    Target::parse("self:a").unwrap(),
                    create_state_from_dependent("self:c")
                ),
                (
                    Target::parse("self:b").unwrap(),
                    create_state_from_dependents(&["self:c", "self:a"])
                )
            ])
        );
    }

    #[tokio::test]
    async fn parent_scope() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["parent/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker
            .track_tasks_by_target(&[Target::parse("parent:child").unwrap()])
            .unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([
                (
                    Target::parse("base:a").unwrap(),
                    create_state_from_dependent("parent:child")
                ),
                (
                    Target::parse("self:b").unwrap(),
                    create_state_from_dependents(&["self:a", "parent:child"])
                ),
                (
                    Target::parse("self:a").unwrap(),
                    create_state_from_dependent("parent:child")
                ),
                (
                    Target::parse("parent:child").unwrap(),
                    create_state_from_file("parent/file.txt")
                ),
            ])
        );
    }

    #[tokio::test]
    async fn marks_dependency_if_dependent_is_touched() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["downstream/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker.with_task_scopes(UpstreamScope::Direct, DownstreamScope::Direct);
        tracker.track_tasks().unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.tasks,
            FxHashMap::from_iter([
                (
                    Target::parse("upstream:task").unwrap(),
                    create_state_from_dependent("downstream:task")
                ),
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

    #[tokio::test]
    async fn marks_dependent_if_dependency_is_touched() {
        let workspace_graph = build_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["upstream/file.txt".into()]);

        let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
        tracker.with_task_scopes(UpstreamScope::Direct, DownstreamScope::Direct);
        tracker.track_tasks().unwrap();
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
                    create_state_from_dependency("upstream:task")
                ),
            ])
        );
    }

    mod task_upstream {
        use super::*;

        #[tokio::test]
        async fn none() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/c.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::None, DownstreamScope::None);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:c").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn direct() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/c.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::Direct, DownstreamScope::None);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:c").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:d").unwrap(),
                        create_state_from_dependent("chain:c")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn direct_no_deps() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/z.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::Direct, DownstreamScope::None);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/z.txt")
                    ),
                    (
                        Target::parse("chain:z").unwrap(),
                        create_state_from_file("chain/z.txt")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn deep() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/c.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::Deep, DownstreamScope::None);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:c").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:d").unwrap(),
                        create_state_from_dependent("chain:c")
                    ),
                    (
                        Target::parse("chain:e").unwrap(),
                        create_state_from_dependent("chain:d")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn deep_no_deps() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/z.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::Deep, DownstreamScope::None);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/z.txt")
                    ),
                    (
                        Target::parse("chain:z").unwrap(),
                        create_state_from_file("chain/z.txt")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn deep_cycle() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["cycle/c.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::Deep, DownstreamScope::None);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("cycle:global").unwrap(),
                        create_state_from_file("cycle/c.txt")
                    ),
                    (Target::parse("cycle:c").unwrap(), {
                        let mut state = create_state_from_file("cycle/c.txt");
                        state.downstream.insert(Target::parse("cycle:b").unwrap());
                        state
                    }),
                    (
                        Target::parse("cycle:a").unwrap(),
                        create_state_from_dependent("cycle:c")
                    ),
                    (
                        Target::parse("cycle:b").unwrap(),
                        create_state_from_dependent("cycle:a")
                    ),
                ])
            );
        }
    }

    mod task_downstream {
        use super::*;

        #[tokio::test]
        async fn none() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/c.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::None, DownstreamScope::None);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:c").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn direct() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/c.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::None, DownstreamScope::Direct);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:c").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:b").unwrap(),
                        create_state_from_dependency("chain:c")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn direct_no_deps() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/z.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::None, DownstreamScope::Direct);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/z.txt")
                    ),
                    (
                        Target::parse("chain:z").unwrap(),
                        create_state_from_file("chain/z.txt")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn deep() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/c.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::None, DownstreamScope::Deep);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:c").unwrap(),
                        create_state_from_file("chain/c.txt")
                    ),
                    (
                        Target::parse("chain:b").unwrap(),
                        create_state_from_dependency("chain:c")
                    ),
                    (
                        Target::parse("chain:a").unwrap(),
                        create_state_from_dependency("chain:b")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn deep_no_deps() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["chain/z.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::None, DownstreamScope::Deep);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("chain:global").unwrap(),
                        create_state_from_file("chain/z.txt")
                    ),
                    (
                        Target::parse("chain:z").unwrap(),
                        create_state_from_file("chain/z.txt")
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn deep_cycle() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["cycle/c.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.with_task_scopes(UpstreamScope::None, DownstreamScope::Deep);
            tracker.track_tasks().unwrap();
            let affected = tracker.build();

            assert_eq!(
                affected.tasks,
                FxHashMap::from_iter([
                    (
                        Target::parse("cycle:global").unwrap(),
                        create_state_from_file("cycle/c.txt")
                    ),
                    (
                        Target::parse("cycle:c").unwrap(),
                        create_state_from_file("cycle/c.txt")
                    ),
                    (
                        Target::parse("cycle:b").unwrap(),
                        create_state_from_dependency("cycle:c")
                    ),
                    (
                        Target::parse("cycle:a").unwrap(),
                        create_state_from_dependency("cycle:b")
                    ),
                ])
            );
        }
    }

    mod ci {
        use super::*;

        #[tokio::test]
        async fn when_ci_tracks_for_true() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.set_ci_check(true);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:enabled").unwrap()])
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

        #[tokio::test]
        async fn when_not_ci_tracks_for_true() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.set_ci_check(false);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:enabled").unwrap()])
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

        #[tokio::test]
        async fn when_ci_doesnt_track_for_false() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.set_ci_check(true);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:disabled").unwrap()])
                .unwrap();
            let affected = tracker.build();

            assert!(affected.tasks.is_empty());
        }

        #[tokio::test]
        async fn when_not_ci_tracks_for_false() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::from_iter(["ci/file.txt".into()]);

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.set_ci_check(false);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:disabled").unwrap()])
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

        #[tokio::test]
        async fn when_ci_always_tracks_if_not_touched() {
            let workspace_graph = build_graph("tasks").await;
            let touched_files = FxHashSet::default();

            let mut tracker = AffectedTracker::new(workspace_graph.into(), touched_files);
            tracker.set_ci_check(true);
            tracker
                .track_tasks_by_target(&[Target::parse("ci:always").unwrap()])
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
    }
}
