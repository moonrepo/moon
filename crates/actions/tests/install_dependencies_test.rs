use moon_action::{Action, ActionStatus, InstallDependenciesNode};
use moon_action_context::ActionContext;
use moon_actions::actions::install_dependencies;
use moon_affected::Affected;
use moon_common::{Id, is_ci, path::WorkspaceRelativePathBuf};
use moon_test_utils::WorkspaceMocker;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use starbase_utils::json::{JsonValue, serde_json};

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
        });

    (sandbox, mocker)
}

async fn run_action(ws: &WorkspaceMocker) -> (Action, ActionStatus) {
    run_action_with_context(ws, ActionContext::default()).await
}

async fn run_action_with_context(
    ws: &WorkspaceMocker,
    context: ActionContext,
) -> (Action, ActionStatus) {
    run_toolchain_action(ws, "tc-tier2", context).await
}

async fn run_toolchain_action(
    ws: &WorkspaceMocker,
    toolchain_id: &str,
    context: ActionContext,
) -> (Action, ActionStatus) {
    let mut action = Action::default();
    let node = InstallDependenciesNode {
        members: None,
        owner_project_id: None,
        project_id: None,
        root: WorkspaceRelativePathBuf::default(),
        toolchain_id: Id::raw(toolchain_id),
    };

    let status = install_dependencies(
        &mut action,
        context.into(),
        ws.mock_app_context().into(),
        ws.mock_workspace_graph().await.into(),
        &node,
    )
    .await
    .unwrap();

    (action, status)
}

// With the vendor directory installed, so that only the fingerprint
// decides whether to install, while in CI it always skips the install
fn create_installed_workspace(
    sandbox: &Sandbox,
    toolchain_id: &str,
    settings: Vec<(&str, JsonValue)>,
) -> WorkspaceMocker {
    sandbox.create_file("vendor/dep/manifest", "");

    WorkspaceMocker::new(sandbox.path())
        .with_default_projects()
        .with_test_toolchains()
        .update_toolchains_config(|config| {
            config
                .plugins
                .get_mut(&Id::raw(toolchain_id))
                .unwrap()
                .config
                .insert("testInstallCommands".into(), JsonValue::Bool(true));

            for (key, value) in &settings {
                let (id, setting) = key.split_once('.').unwrap();

                config
                    .plugins
                    .get_mut(&Id::raw(id))
                    .unwrap()
                    .config
                    .insert(setting.into(), value.to_owned());
            }
        })
}

fn installed_status() -> ActionStatus {
    if is_ci() {
        ActionStatus::Skipped
    } else {
        ActionStatus::Passed
    }
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

    // More than one lockfile can exist, like one that another package manager
    // left behind, and a change to any of them must install again
    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn installs_when_any_lockfile_changes() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("tc.lock", "v1");
        sandbox.create_file("tc.extra.lock", "v1");
        sandbox.create_file("vendor/dep/manifest", "");

        let ws = WorkspaceMocker::new(sandbox.path())
            .with_default_projects()
            .with_test_toolchains()
            .update_toolchains_config(|config| {
                let plugin = config.plugins.get_mut(&Id::raw("tc-tier2")).unwrap();

                plugin
                    .config
                    .insert("testInstallCommands".into(), JsonValue::Bool(true));
                plugin.config.insert(
                    "testLockFileNames".into(),
                    serde_json::json!(["tc.lock", "tc.extra.lock"]),
                );
            });

        run_action(&ws).await;

        // Nothing changed
        let (_, status) = run_action(&ws).await;

        assert_eq!(status, ActionStatus::Skipped);

        // Only the lockfile that isn't first changed
        sandbox.create_file("tc.extra.lock", "v2");

        let (_, status) = run_action(&ws).await;

        assert_eq!(
            status,
            if is_ci() {
                ActionStatus::Skipped
            } else {
                ActionStatus::Passed
            }
        );
    }

    // Like the package manager's arguments for JavaScript
    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn installs_when_a_required_toolchains_config_changes() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("tc.lock", "v1");

        let mut statuses = vec![];

        for args in ["a", "a", "b"] {
            let ws = create_installed_workspace(
                &sandbox,
                "tc-tier2-reqs",
                vec![("tc-tier3.installArgs", serde_json::json!([args]))],
            );

            let (_, status) =
                run_toolchain_action(&ws, "tc-tier2-reqs", ActionContext::default()).await;

            statuses.push(status);
        }

        assert_eq!(
            statuses,
            [
                installed_status(),
                ActionStatus::Skipped,
                installed_status()
            ]
        );
    }

    // Like `.npmrc`, `bunfig.toml`, or `.cargo/config.toml`, but only those
    // the globs match from the root
    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn installs_when_a_config_file_changes() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("tc.lock", "v1");
        sandbox.create_file("tc.config", "a");
        sandbox.create_file("extra.tcrc", "a");
        sandbox.create_file(".tc/config.toml", "a");
        sandbox.create_file("nested/tc.config", "a");

        let ws = create_installed_workspace(
            &sandbox,
            "tc-tier2",
            vec![(
                "tc-tier2.testConfigFileGlobs",
                serde_json::json!(["tc.config", "*.tcrc", ".tc/*.toml"]),
            )],
        );

        let mut statuses = vec![];

        for (file, content) in [
            ("tc.config", "a"),
            ("tc.config", "a"),
            ("nested/tc.config", "b"),
            ("extra.tcrc", "b"),
            (".tc/config.toml", "b"),
        ] {
            sandbox.create_file(file, content);

            let (_, status) = run_action(&ws).await;

            statuses.push(status);
        }

        assert_eq!(
            statuses,
            [
                installed_status(),
                ActionStatus::Skipped,
                ActionStatus::Skipped,
                installed_status(),
                installed_status()
            ]
        );
    }

    // Like a local path or a Git repository, which have no version
    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn installs_when_a_dependency_without_a_version_changes() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("tc.lock", "v1");

        let ws = create_installed_workspace(&sandbox, "tc-tier2", vec![]);

        // Manifests are only parsed when they changed
        let context = || ActionContext {
            changed_files: [WorkspaceRelativePathBuf::from("tc.cfg")]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        let mut statuses = vec![];

        for (left, right) in [
            ("path:../left2", "url:https://host/right.git"),
            ("path:../left2", "url:https://host/right.git"),
            ("path:../left", "url:https://host/right.git"),
            ("path:../left", "url:https://host/right2.git"),
        ] {
            sandbox.create_file("tc.cfg", format!("dep left {left}\ndep right {right}\n"));

            let (_, status) = run_action_with_context(&ws, context()).await;

            statuses.push(status);
        }

        assert_eq!(
            statuses,
            [
                installed_status(),
                ActionStatus::Skipped,
                installed_status(),
                installed_status()
            ]
        );
    }

    mod affected {
        use super::*;

        fn create_affected_context() -> ActionContext {
            ActionContext {
                affected: Some(Affected::default()),
                ..Default::default()
            }
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn installs_when_dependencies_missing() {
            let (_sandbox, ws) = create_workspace();

            // A fresh clone or worktree has no vendor directory, so the
            // tasks would fail without an install, even in affected mode
            let (action, status) = run_action_with_context(&ws, create_affected_context()).await;

            assert_eq!(status, ActionStatus::Passed);
            assert_eq!(count_execs(&action), 1);
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn skips_when_dependencies_installed() {
            let (sandbox, ws) = create_workspace();
            sandbox.create_file("vendor/dep/manifest", "");

            let (action, status) = run_action_with_context(&ws, create_affected_context()).await;

            assert_eq!(status, ActionStatus::Skipped);
            assert_eq!(count_execs(&action), 0);
        }

        #[serial_test::serial]
        #[tokio::test(flavor = "multi_thread")]
        async fn skips_when_dependencies_installed_and_lockfile_changed() {
            let (sandbox, ws) = create_workspace();

            // Install for the first time
            let (_, status) = run_action_with_context(&ws, create_affected_context()).await;

            assert_eq!(status, ActionStatus::Passed);

            // Simulate the install having populated the vendor directory,
            // and the lockfile changing afterwards
            sandbox.create_file("vendor/dep/manifest", "");
            sandbox.create_file("tc.lock", "v2");

            let (action, status) = run_action_with_context(&ws, create_affected_context()).await;

            assert_eq!(status, ActionStatus::Skipped);
            assert_eq!(count_execs(&action), 0);
        }
    }
}
