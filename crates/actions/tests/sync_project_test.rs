use moon_action::{Action, ActionStatus, SyncProjectNode};
use moon_action_context::ActionContext;
use moon_actions::actions::sync_project;
use moon_common::{Id, is_ci};
use moon_env_var::GlobalEnvBag;
use moon_test_utils2::WorkspaceMocker;
use starbase_sandbox::{Sandbox, create_sandbox};

fn create_workspace() -> (Sandbox, WorkspaceMocker) {
    let sandbox = create_sandbox("projects");
    let mocker = WorkspaceMocker::new(sandbox.path())
        .with_default_projects()
        .with_test_toolchains();

    (sandbox, mocker)
}

mod sync_project {
    use super::*;

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn can_skip_action() {
        let (_sandbox, ws) = create_workspace();
        let mut action = Action::default();

        let bag = GlobalEnvBag::instance();
        bag.set("MOON_SKIP_SYNC_PROJECT", "true");

        let status = sync_project(
            &mut action,
            ActionContext::default().into(),
            ws.mock_app_context().into(),
            ws.mock_workspace_graph().await.into(),
            &SyncProjectNode {
                project_id: Id::raw("a"),
            },
        )
        .await
        .unwrap();

        bag.remove("MOON_SKIP_SYNC_PROJECT");

        assert_eq!(status, ActionStatus::Skipped);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn can_skip_action_by_id() {
        let (_sandbox, ws) = create_workspace();
        let mut action = Action::default();

        let bag = GlobalEnvBag::instance();
        bag.set("MOON_SKIP_SYNC_PROJECT", "a");

        let status = sync_project(
            &mut action,
            ActionContext::default().into(),
            ws.mock_app_context().into(),
            ws.mock_workspace_graph().await.into(),
            &SyncProjectNode {
                project_id: Id::raw("a"),
            },
        )
        .await
        .unwrap();

        bag.remove("MOON_SKIP_SYNC_PROJECT");

        assert_eq!(status, ActionStatus::Skipped);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_skip_action_if_id_not_match() {
        let (_sandbox, ws) = create_workspace();
        let mut action = Action::default();

        let bag = GlobalEnvBag::instance();
        bag.set("MOON_SKIP_SYNC_PROJECT", "c");

        let status = sync_project(
            &mut action,
            ActionContext::default().into(),
            ws.mock_app_context().into(),
            ws.mock_workspace_graph().await.into(),
            &SyncProjectNode {
                project_id: Id::raw("a"),
            },
        )
        .await
        .unwrap();

        bag.remove("MOON_SKIP_SYNC_PROJECT");

        assert_eq!(status, ActionStatus::Passed);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn creates_a_snapshot() {
        let (sandbox, ws) = create_workspace();
        let mut action = Action::default();
        let snapshot_file = sandbox.path().join(".moon/cache/states/a/snapshot.json");

        assert!(!snapshot_file.exists());

        let status = sync_project(
            &mut action,
            ActionContext::default().into(),
            ws.mock_app_context().into(),
            ws.mock_workspace_graph().await.into(),
            &SyncProjectNode {
                project_id: Id::raw("a"),
            },
        )
        .await
        .unwrap();

        assert!(snapshot_file.exists());
        assert_eq!(status, ActionStatus::Passed);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn creates_a_snapshot_even_if_skipped() {
        let (sandbox, ws) = create_workspace();
        let mut action = Action::default();
        let snapshot_file = sandbox.path().join(".moon/cache/states/a/snapshot.json");

        assert!(!snapshot_file.exists());

        let bag = GlobalEnvBag::instance();
        bag.set("MOON_SKIP_SYNC_PROJECT", "true");

        let status = sync_project(
            &mut action,
            ActionContext::default().into(),
            ws.mock_app_context().into(),
            ws.mock_workspace_graph().await.into(),
            &SyncProjectNode {
                project_id: Id::raw("a"),
            },
        )
        .await
        .unwrap();

        assert!(snapshot_file.exists());

        bag.remove("MOON_SKIP_SYNC_PROJECT");

        assert_eq!(status, ActionStatus::Skipped);
    }

    mod toolchains {
        use super::*;

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_run_if_not_configured() {
            let (_sandbox, ws) = create_workspace();
            let mut action = Action::default();

            let status = sync_project(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                &SyncProjectNode {
                    project_id: Id::raw("a"),
                },
            )
            .await
            .unwrap();

            assert_eq!(status, ActionStatus::Passed);

            assert!(
                !action
                    .operations
                    .iter()
                    .any(|op| op.plugin.as_ref().is_some_and(|id| id.starts_with("tc-")))
            );
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_run_if_disabled_by_override() {
            let (_sandbox, ws) = create_workspace();
            let mut action = Action::default();

            let status = sync_project(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                &SyncProjectNode {
                    project_id: Id::raw("c"),
                },
            )
            .await
            .unwrap();

            assert_eq!(status, ActionStatus::Passed);

            assert!(
                !action
                    .operations
                    .iter()
                    .any(|op| op.plugin.as_ref().is_some_and(|id| id.starts_with("tc-")))
            );
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn runs_when_enabled() {
            let (sandbox, ws) = create_workspace();
            let mut action = Action::default();

            let status = sync_project(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                &SyncProjectNode {
                    project_id: Id::raw("b"),
                },
            )
            .await
            .unwrap();

            // Is invalid in CI because files changed
            assert_eq!(
                status,
                if is_ci() {
                    ActionStatus::Invalid
                } else {
                    ActionStatus::Passed
                }
            );

            // All toolchains inherit from tc-tier1
            assert_eq!(
                action
                    .operations
                    .iter()
                    .filter_map(|op| op.plugin.as_ref().map(|id| id.as_str()))
                    .collect::<Vec<_>>(),
                ["tc-tier1"]
            );

            // Verify operation and changed files
            let op = action
                .operations
                .iter()
                .find(|op| op.plugin.as_ref().is_some_and(|id| id == "tc-tier1"))
                .unwrap();

            assert!(
                op.get_file_state()
                    .unwrap()
                    .changed_files
                    .contains(&sandbox.path().join("file.txt"))
            );

            let nested_op = op.operations.first().unwrap();

            assert_eq!(nested_op.status, ActionStatus::Failed);
            assert_eq!(nested_op.id.as_ref().unwrap(), "sync-project-test");
        }
    }
}
