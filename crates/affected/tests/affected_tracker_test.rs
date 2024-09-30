use moon_affected::*;
use moon_common::Id;
use moon_task::Target;
use moon_test_utils2::generate_project_graph;
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;

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
        let project_graph = generate_project_graph("projects").await;
        let touched_files = FxHashSet::default();

        let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
        tracker.track_projects().unwrap();
        let affected = tracker.build();

        assert!(affected.projects.is_empty());
    }

    #[tokio::test]
    async fn tracks_projects() {
        let project_graph = generate_project_graph("projects").await;
        let touched_files = FxHashSet::from_iter(["a/file.txt".into()]);

        let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
        let project_graph = generate_project_graph("projects").await;
        let touched_files = FxHashSet::from_iter([
            "a/file.txt".into(),
            "b/file.txt".into(),
            "e/file.txt".into(),
        ]);

        let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
        tracker.with_project_scopes(UpstreamScope::None, DownstreamScope::None);
        tracker.track_projects().unwrap();
        let affected = tracker.build();

        assert_eq!(
            affected.projects,
            FxHashMap::from_iter([
                (Id::raw("a"), create_state_from_file("a/file.txt")),
                (Id::raw("b"), create_state_from_file("b/file.txt")),
                (Id::raw("e"), create_state_from_file("e/file.txt")),
                (Id::raw("root"), create_state_from_file("e/file.txt")),
            ])
        );
    }

    mod project_upstream {
        use super::*;

        #[tokio::test]
        async fn none() {
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["a/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["a/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["e/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["a/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["e/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
    }

    mod project_downstream {
        use super::*;

        #[tokio::test]
        async fn none() {
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["c/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["c/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["e/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["c/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
            let project_graph = generate_project_graph("projects").await;
            let touched_files = FxHashSet::from_iter(["e/file.txt".into()]);

            let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
    }
}

mod affected_tasks {
    use super::*;
    use moon_test_utils2::pretty_assertions::assert_eq;

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

    // fn create_state_from_dependency(id: &str) -> AffectedTaskState {
    //     let mut state = AffectedTaskState::default();
    //     state.upstream.insert(Target::parse(id).unwrap());
    //     state
    // }

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
        let project_graph = generate_project_graph("tasks").await;
        let touched_files = FxHashSet::default();

        let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
        tracker.track_projects().unwrap();
        let affected = tracker.build();

        assert!(affected.tasks.is_empty());
    }

    #[tokio::test]
    async fn self_scope() {
        let project_graph = generate_project_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["self/file.txt".into()]);

        let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
        let project_graph = generate_project_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["parent/file.txt".into()]);

        let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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
    async fn by_env_var() {
        let project_graph = generate_project_graph("tasks").await;
        let touched_files = FxHashSet::default();

        env::set_var("ENV", "affected");

        let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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

        env::remove_var("ENV");
    }

    #[tokio::test]
    async fn marks_dependency_if_dependent_is_touched() {
        let project_graph = generate_project_graph("tasks").await;
        let touched_files = FxHashSet::from_iter(["downstream/file.txt".into()]);

        let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
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

    // #[tokio::test]
    // async fn marks_dependent_if_dependency_is_touched() {
    //     let project_graph = generate_project_graph("tasks").await;
    //     let touched_files = FxHashSet::from_iter(["upstream/file.txt".into()]);

    //     let mut tracker = AffectedTracker::new(&project_graph, &touched_files);
    //     tracker.track_tasks().unwrap();
    //     let affected = tracker.build();

    //     assert_eq!(
    //         affected.tasks,
    //         FxHashMap::from_iter([
    //             (
    //                 Target::parse("upstream:task").unwrap(),
    //                 create_state_from_file("upstream/file.txt")
    //             ),
    //             (
    //                 Target::parse("upstream:global").unwrap(),
    //                 create_state_from_file("upstream/file.txt")
    //             ),
    //             (
    //                 Target::parse("downstream:task").unwrap(),
    //                 create_state_from_dependency("upstream:task")
    //             ),
    //         ])
    //     );
    // }
}
