use moon_action::{Action, ActionStatus, InstallDependenciesNode};
use moon_action_context::ActionContext;
use moon_actions::actions::install_dependencies;
use moon_common::{Id, is_ci, path::WorkspaceRelativePathBuf};
use moon_test_utils::WorkspaceMocker;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use starbase_utils::json::JsonValue;

fn create_workspace() -> (Sandbox, WorkspaceMocker) {
    let sandbox = create_empty_sandbox();

    // The tier1 toolchain (inherited by tc-tier2) registers `tc.lock`
    // as its lockfile and `vendor` as its vendor directory
    sandbox.create_file("tc.lock", "v1");

    let mocker = WorkspaceMocker::new(sandbox.path())
        .with_default_projects()
        .with_test_toolchains()
        // Opt-in to the install/dedupe commands returned by tc-tier2
        .update_toolchains_config(|config| {
            config
                .plugins
                .get_mut(&Id::raw("tc-tier2"))
                .unwrap()
                .config
                .insert("testInstallCommands".into(), JsonValue::Bool(true));
        })
        // Hash with the cache engine instead of the VCS,
        // as the sandbox is not a git repository
        .update_workspace_config(|config| {
            config.experiments.native_file_hashing = true;
        });

    (sandbox, mocker)
}

async fn run_action(ws: &WorkspaceMocker) -> (Action, ActionStatus) {
    let mut action = Action::default();
    let node = InstallDependenciesNode {
        members: None,
        project_id: None,
        root: WorkspaceRelativePathBuf::default(),
        toolchain_id: Id::raw("tc-tier2"),
    };

    let status = install_dependencies(
        &mut action,
        ActionContext::default().into(),
        ws.mock_app_context().into(),
        ws.mock_workspace_graph().await.into(),
        &node,
    )
    .await
    .unwrap();

    (action, status)
}

fn count_execs(action: &Action) -> usize {
    action
        .operations
        .iter()
        .filter(|op| op.meta.is_process_execution())
        .count()
}

mod install_dependencies {
    use super::*;

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn does_not_dedupe_on_first_install() {
        let (_sandbox, ws) = create_workspace();

        // No vendor directory exists yet, so only the install command must
        // run. Deduping would rewrite a pristine lockfile, since a cold
        // cache always hashes as "changed"
        let (action, status) = run_action(&ws).await;

        assert_eq!(status, ActionStatus::Passed);
        assert_eq!(count_execs(&action), 1);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn dedupes_when_dependencies_already_installed() {
        let (sandbox, ws) = create_workspace();
        sandbox.create_file("vendor/dep/manifest", "");

        let (action, status) = run_action(&ws).await;

        if is_ci() {
            // In CI, an existing vendor directory skips the action entirely
            assert_eq!(status, ActionStatus::Skipped);
            assert_eq!(count_execs(&action), 0);
        } else {
            // Install + dedupe
            assert_eq!(status, ActionStatus::Passed);
            assert_eq!(count_execs(&action), 2);
        }
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn dedupes_on_lockfile_change_after_first_install() {
        let (sandbox, ws) = create_workspace();

        // Cold cache: install only, no dedupe
        let (action, status) = run_action(&ws).await;

        assert_eq!(status, ActionStatus::Passed);
        assert_eq!(count_execs(&action), 1);

        // Simulate the install having populated the vendor directory
        sandbox.create_file("vendor/dep/manifest", "");

        // Nothing changed: nothing to install or dedupe
        let (action, status) = run_action(&ws).await;

        assert_eq!(status, ActionStatus::Skipped);
        assert_eq!(count_execs(&action), 0);

        // Lockfile changed: install and dedupe
        sandbox.create_file("tc.lock", "v2");

        let (action, status) = run_action(&ws).await;

        if is_ci() {
            assert_eq!(status, ActionStatus::Skipped);
            assert_eq!(count_execs(&action), 0);
        } else {
            assert_eq!(status, ActionStatus::Passed);
            assert_eq!(count_execs(&action), 2);
        }
    }
}
