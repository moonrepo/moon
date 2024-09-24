use moon_affected::*;
use moon_common::Id;
use moon_test_utils2::generate_project_graph;
use rustc_hash::{FxHashMap, FxHashSet};

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
    state.upstream.extend(ids.iter().map(|id| Id::raw(id)));
    state
}

fn create_state_from_dependent(id: &str) -> AffectedProjectState {
    let mut state = AffectedProjectState::default();
    state.downstream.insert(Id::raw(id));
    state
}

fn create_state_from_dependents(ids: &[&str]) -> AffectedProjectState {
    let mut state = AffectedProjectState::default();
    state.downstream.extend(ids.iter().map(|id| Id::raw(id)));
    state
}

mod affected_tracker {
    use super::*;

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
