use moon_app_context::AppContext;
use moon_common::Id;
use moon_env_var::GlobalEnvBag;
use moon_pdk_api::SetupToolchainInput;
use moon_process::Command;
use moon_process_augment::AugmentedCommand;
use moon_test_utils::WorkspaceMocker;
use moon_toolchain_plugin::ToolchainPlugin;
use proto_core::UnresolvedVersionSpec;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::ffi::OsString;
use std::fs;
use std::sync::Arc;

// The `tc-tier3-tool` test toolchain registers a real proto tool, and is
// configured with a version, but is never installed within the sandbox
fn create_workspace() -> (Sandbox, WorkspaceMocker) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path())
        .with_test_toolchains()
        .update_toolchains_config(|config| {
            config.plugins.entry(Id::raw("tc-tier3-tool")).or_default();
        });

    (sandbox, mocker)
}

// Pretend the tool was previously installed by proto, so that setting up the
// toolchain resolves and locates it instead of attempting a real install
fn install_toolchain_tool(sandbox: &Sandbox, with_exe: bool) {
    let dir = sandbox.path().join(".proto/tools/tc-tier3-tool");

    fs::create_dir_all(dir.join("1.2.3/bin")).unwrap();
    fs::write(
        dir.join("manifest.json"),
        r#"{ "installed_versions": ["1.2.3"] }"#,
    )
    .unwrap();

    if with_exe {
        fs::write(dir.join("1.2.3/bin/tc-tool"), "").unwrap();
    }
}

async fn setup_toolchain(app_context: &AppContext) -> Arc<ToolchainPlugin> {
    let toolchain = app_context
        .toolchain_registry
        .load("tc-tier3-tool")
        .await
        .unwrap();

    toolchain
        .setup_toolchain(
            SetupToolchainInput {
                configured_version: UnresolvedVersionSpec::parse("1.2.3").ok(),
                context: app_context.toolchain_registry.create_context(),
                ..Default::default()
            },
            None,
            || Ok(()),
        )
        .await
        .unwrap();

    toolchain
}

fn get_env<'a>(command: &'a Command, key: &str) -> Option<&'a str> {
    command
        .env
        .get(&OsString::from(key))
        .map(|value| value.get_value().unwrap().to_str().unwrap())
}

mod augmented_command {
    use super::*;

    mod inherit_from_toolchains {
        use super::*;

        // Setting up a toolchain is what puts its executables on disk, so a
        // toolchain that was merely declared in the workspace cannot be located.
        // https://github.com/moonrepo/moon/issues/2691
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_locate_toolchains_that_have_not_been_setup() {
            let (_sandbox, mocker) = create_workspace();
            let app_context = mocker.mock_app_context();
            let bag = GlobalEnvBag::default();

            let mut command = AugmentedCommand::new(&app_context, &bag, "noop");

            // Would fail with `proto::locate::missing_executable` if we
            // attempted to locate the uninstalled toolchain
            command.inherit_from_toolchains(None, None).await.unwrap();

            let command = command.augment();

            assert!(command.paths.is_empty());
        }

        // The version is what `proto` uses to resolve the tool at runtime, and
        // is safe to inherit even when the tool hasn't been installed yet
        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_versions_of_toolchains_that_have_not_been_setup() {
            let (_sandbox, mocker) = create_workspace();
            let app_context = mocker.mock_app_context();
            let bag = GlobalEnvBag::default();

            let mut command = AugmentedCommand::new(&app_context, &bag, "noop");
            command.inherit_from_toolchains(None, None).await.unwrap();

            let command = command.augment();

            assert_eq!(
                get_env(&command, "PROTO_TC_TIER3_TOOL_VERSION"),
                Some("1.2.3")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_toolchains_that_have_been_setup() {
            let (sandbox, mocker) = create_workspace();

            install_toolchain_tool(&sandbox, true);

            let app_context = mocker.mock_app_context();
            let bag = GlobalEnvBag::default();
            let toolchain = setup_toolchain(&app_context).await;

            assert!(toolchain.is_setup());

            let mut command = AugmentedCommand::new(&app_context, &bag, "noop");
            command.inherit_from_toolchains(None, None).await.unwrap();

            let command = command.augment();

            let bin_dir = sandbox.path().join(".proto/tools/tc-tier3-tool/1.2.3/bin");

            assert!(command.paths.contains(&bin_dir.into_os_string()));
        }

        // Guards the tests above: locating this toolchain must actually be
        // capable of failing, otherwise they would pass for the wrong reason
        // if the plugin ever stopped registering a proto tool
        #[tokio::test(flavor = "multi_thread")]
        async fn locating_a_toolchain_without_executables_fails() {
            let (sandbox, mocker) = create_workspace();

            install_toolchain_tool(&sandbox, false);

            let app_context = mocker.mock_app_context();
            let toolchain = setup_toolchain(&app_context).await;

            assert!(toolchain.is_setup());

            let error = toolchain
                .get_command_paths(UnresolvedVersionSpec::parse("1.2.3").ok())
                .await
                .unwrap_err();

            assert!(error.to_string().contains("Unable to find an executable"));
        }
    }
}
