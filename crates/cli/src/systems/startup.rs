use moon_app_components::{ExtensionRegistry, MoonEnv, ProtoEnv, WorkspaceRoot};
use moon_common::{consts::PROTO_CLI_VERSION, is_test_env, is_unformatted_stdout, path::exe_name};
use moon_env::MoonEnvironment;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_workspace::Workspace;
use proto_core::{is_offline, ProtoEnvironment, ProtoError};
use proto_installer::*;
use starbase::system;
use std::env;
use std::sync::Arc;
use tracing::debug;

// IN ORDER:

#[system]
pub async fn load_environments(states: StatesMut) {
    states.set(MoonEnv(Arc::new(MoonEnvironment::new()?)));
    states.set(ProtoEnv(Arc::new(ProtoEnvironment::new()?)));
}

#[system]
pub async fn load_workspace(states: StatesMut, resources: ResourcesMut) {
    let workspace = moon::load_workspace_from(Arc::clone(states.get::<ProtoEnv>())).await?;

    states.set(WorkspaceRoot(workspace.root.clone()));

    // Ensure our env instance is using the found workspace root,
    // as this is required for plugins to function entirely!
    Arc::get_mut(states.get_mut::<MoonEnv>())
        .unwrap()
        .workspace_root = workspace.root.clone();

    resources.set(workspace);
}

#[system]
pub async fn create_plugin_registries(
    resources: ResourcesMut,
    moon_env: StateRef<MoonEnv>,
    proto_env: StateRef<ProtoEnv>,
) {
    resources.set(ExtensionRegistry::new(
        Arc::clone(moon_env),
        Arc::clone(proto_env),
    ));
}

#[system]
pub async fn install_proto(proto_env: StateRef<ProtoEnv>, workspace: ResourceRef<Workspace>) {
    let bin_name = exe_name("proto");
    let install_dir = proto_env.tools_dir.join("proto").join(PROTO_CLI_VERSION);

    debug!(proto = ?install_dir.join(&bin_name), "Checking if proto is installed");

    // Set the version so that proto lookup paths take it into account
    env::set_var("PROTO_VERSION", PROTO_CLI_VERSION);
    env::set_var("PROTO_IGNORE_MIGRATE_WARNING", "true");
    env::set_var("PROTO_VERSION_CHECK", "false");

    // This causes a ton of issues when running the test suite,
    // so just avoid it and assume proto exists!
    if install_dir.exists() || is_test_env() || !workspace.toolchain_config.should_install_proto() {
        return Ok(());
    }

    debug!("Installing proto");

    if is_unformatted_stdout() {
        print_checkpoint(
            format!("installing proto {}", PROTO_CLI_VERSION),
            Checkpoint::Setup,
        );
    }

    // If offline but a primary proto binary exists,
    // use that instead of failing, even if a different version!
    if is_offline() {
        let existing_bin = proto_env.bin_dir.join(&bin_name);

        if existing_bin.exists() {
            debug!(
                proto = ?existing_bin,
                "No internet connection, but using existing {} binary",
                bin_name
            );

            return Ok(());
        } else {
            return Err(ProtoError::InternetConnectionRequired.into());
        }
    }

    let target_triple = determine_triple()?;

    debug!("Downloading proto archive ({})", target_triple);

    let result = download_release(
        &target_triple,
        PROTO_CLI_VERSION,
        &proto_env.temp_dir,
        |_, _| {},
    )
    .await?;

    debug!("Unpacking archive and installing proto");

    unpack_release(result, &install_dir, &proto_env.temp_dir)?;

    debug!("Successfully installed proto!");
}
