use moon_pdk_api::{ExtendTaskCommandInput, SyncWorkspaceInput};
use moon_test_utils::WorkspaceMocker;
use starbase_sandbox::{Sandbox, create_empty_sandbox};

fn create_workspace() -> (Sandbox, WorkspaceMocker) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path()).with_test_extensions();

    (sandbox, mocker)
}

mod extension_plugin {
    use super::*;

    mod guarded_funcs {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn execute_is_a_noop_when_func_missing() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_extension_registry();
            let extension = registry.load("ext-sync").await.unwrap();

            // `ext-sync` doesn't export `execute_extension`
            extension
                .execute(vec!["--arg".into()], registry.create_context())
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn define_extension_config_returns_none_when_func_missing() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_extension_registry();
            let extension = registry.load("ext-sync").await.unwrap();

            assert!(extension.define_extension_config().await.unwrap().is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extend_task_command_returns_default_when_func_missing() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_extension_registry();
            let extension = registry.load("ext-sync").await.unwrap();

            let output = extension
                .extend_task_command(ExtendTaskCommandInput {
                    context: registry.create_context(),
                    ..Default::default()
                })
                .await
                .unwrap();

            assert!(output.command.is_none());
            assert!(output.paths.is_empty());
        }
    }

    mod sync_workspace {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_changed_files_to_real_paths() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_extension_registry();
            let extension = registry.load("ext-sync").await.unwrap();

            let output = extension
                .sync_workspace(SyncWorkspaceInput {
                    context: registry.create_context(),
                    ..Default::default()
                })
                .await
                .unwrap();

            assert_eq!(output.operations.len(), 1);
            assert_eq!(output.changed_files.len(), 1);

            // The virtual `/workspace` path is converted back to a real path
            let file = &output.changed_files[0];

            assert_eq!(file.as_path(), ws.workspace_root.join("file-ext.txt"));
        }
    }
}
