use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_actions::actions::sync_workspace;
use moon_env_var::GlobalEnvBag;
use moon_test_utils2::WorkspaceMocker;
use starbase_sandbox::{Sandbox, create_empty_sandbox};

fn create_workspace() -> (Sandbox, WorkspaceMocker) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path())
        .with_default_projects()
        .with_test_toolchains();

    (sandbox, mocker)
}

mod sync_workspace {
    use super::*;

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn can_skip_action() {
        let (_, ws) = create_workspace();
        let mut action = Action::default();

        let bag = GlobalEnvBag::instance();
        bag.set("MOON_SKIP_SYNC_WORKSPACE", "true");

        let status = sync_workspace(
            &mut action,
            ActionContext::default().into(),
            ws.mock_app_context().into(),
            ws.mock_workspace_graph().await.into(),
            ws.mock_toolchain_registry().into(),
        )
        .await
        .unwrap();

        bag.remove("MOON_SKIP_SYNC_WORKSPACE");

        assert_eq!(status, ActionStatus::Skipped);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn runs_all_toolchains() {
        let (sandbox, ws) = create_workspace();
        let mut action = Action::default();

        let status = sync_workspace(
            &mut action,
            ActionContext::default().into(),
            ws.mock_app_context().into(),
            ws.mock_workspace_graph().await.into(),
            ws.mock_toolchain_registry().into(),
        )
        .await
        .unwrap();

        assert_eq!(status, ActionStatus::Passed);

        // All toolchains inherit from tc-tier1
        let mut ids = action
            .operations
            .iter()
            .filter_map(|op| op.plugin.as_ref().map(|id| id.as_str()))
            .collect::<Vec<_>>();
        ids.sort();

        assert_eq!(
            ids,
            [
                "tc-tier1",
                "tc-tier2",
                "tc-tier2-reqs",
                "tc-tier2-setup-env",
                "tc-tier3",
                "tc-tier3-reqs"
            ]
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
        assert_eq!(nested_op.id.as_ref().unwrap(), "sync-workspace-test");
    }

    mod config_schemas {
        use super::*;

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn creates_schemas() {
            let (sandbox, ws) = create_workspace();
            let mut action = Action::default();
            let schemas_dir = sandbox.path().join(".moon/cache/schemas");

            assert!(!schemas_dir.exists());

            let status = sync_workspace(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                ws.mock_toolchain_registry().into(),
            )
            .await
            .unwrap();

            assert!(schemas_dir.exists());

            assert_eq!(status, ActionStatus::Passed);
        }
    }

    mod codeowners {
        use super::*;

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_if_not_configured() {
            let (sandbox, ws) = create_workspace();
            let mut action = Action::default();
            let code_file = sandbox.path().join(".github/CODEOWNERS");

            assert!(!code_file.exists());

            let status = sync_workspace(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                ws.mock_toolchain_registry().into(),
            )
            .await
            .unwrap();

            assert!(!code_file.exists());

            assert_eq!(status, ActionStatus::Passed);
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn creates_if_configured() {
            let (sandbox, mut ws) = create_workspace();
            let mut action = Action::default();
            let code_file = sandbox.path().join(".github/CODEOWNERS");

            assert!(!code_file.exists());

            ws.workspace_config.codeowners.sync_on_run = true;

            let status = sync_workspace(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                ws.mock_toolchain_registry().into(),
            )
            .await
            .unwrap();

            assert!(code_file.exists());

            assert_eq!(status, ActionStatus::Passed);
        }
    }

    mod vcs_hooks {
        use super::*;

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_if_not_configured() {
            let (sandbox, ws) = create_workspace();
            sandbox.enable_git();

            let mut action = Action::default();
            let hooks_dir = sandbox.path().join(".moon/hooks");

            assert!(!hooks_dir.exists());

            let status = sync_workspace(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                ws.mock_toolchain_registry().into(),
            )
            .await
            .unwrap();

            assert!(!hooks_dir.exists());

            assert_eq!(status, ActionStatus::Passed);
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_if_configured_but_not_syncing() {
            let (sandbox, mut ws) = create_workspace();
            sandbox.enable_git();

            let mut action = Action::default();
            let hooks_dir = sandbox.path().join(".moon/hooks");

            assert!(!hooks_dir.exists());

            ws.workspace_config
                .vcs
                .hooks
                .insert("pre-commit".into(), vec!["do something".into()]);

            let status = sync_workspace(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                ws.mock_toolchain_registry().into(),
            )
            .await
            .unwrap();

            assert!(!hooks_dir.exists());

            assert_eq!(status, ActionStatus::Passed);
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_if_enabled_but_no_vcs() {
            let (sandbox, mut ws) = create_workspace();
            let mut action = Action::default();
            let hooks_dir = sandbox.path().join(".moon/hooks");

            assert!(!hooks_dir.exists());

            ws.workspace_config.vcs.sync_hooks = true;
            ws.workspace_config
                .vcs
                .hooks
                .insert("pre-commit".into(), vec!["do something".into()]);

            let status = sync_workspace(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                ws.mock_toolchain_registry().into(),
            )
            .await
            .unwrap();

            assert!(!hooks_dir.exists());

            assert_eq!(status, ActionStatus::Passed);
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn creates_if_enabled() {
            let (sandbox, mut ws) = create_workspace();
            sandbox.enable_git();

            let mut action = Action::default();
            let hooks_dir = sandbox.path().join(".moon/hooks");

            assert!(!hooks_dir.exists());

            ws.workspace_config.vcs.sync_hooks = true;
            ws.workspace_config
                .vcs
                .hooks
                .insert("pre-commit".into(), vec!["do something".into()]);

            let status = sync_workspace(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                ws.mock_toolchain_registry().into(),
            )
            .await
            .unwrap();

            assert!(hooks_dir.exists());

            assert_eq!(status, ActionStatus::Passed);
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn captures_an_operation() {
            let (sandbox, mut ws) = create_workspace();
            sandbox.enable_git();

            let mut action = Action::default();

            ws.workspace_config.vcs.sync_hooks = true;
            ws.workspace_config
                .vcs
                .hooks
                .insert("pre-commit".into(), vec!["do something".into()]);

            let status = sync_workspace(
                &mut action,
                ActionContext::default().into(),
                ws.mock_app_context().into(),
                ws.mock_workspace_graph().await.into(),
                ws.mock_toolchain_registry().into(),
            )
            .await
            .unwrap();

            assert_eq!(status, ActionStatus::Passed);
            assert!(
                action
                    .operations
                    .iter()
                    .any(|op| op.id.as_ref().is_some_and(|id| id == "vcs-hooks"))
            );
        }
    }
}
