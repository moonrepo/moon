use crate::app::GlobalArgs;
use moon_app_components::{Console, ExtensionRegistry, MoonEnv, ProtoEnv, WorkspaceRoot};
use moon_common::{consts::PROTO_CLI_VERSION, is_test_env, path::exe_name};
use moon_console::Checkpoint;
use moon_env::MoonEnvironment;
use moon_platform_plugin::PlatformRegistry;
use moon_plugin::{PluginRegistry, PluginType};
use moon_workspace::Workspace;
use proto_core::{is_offline, ProtoEnvironment, ProtoError};
use proto_installer::*;
use rustc_hash::FxHashMap;
use starbase::system;
use std::env;
use std::sync::Arc;
use tracing::debug;

// IN ORDER:

#[system]
pub async fn load_environments(states: StatesMut, resources: ResourcesMut) {
    let quiet = { states.get::<GlobalArgs>().quiet };

    states.set(MoonEnv(Arc::new(MoonEnvironment::new()?)));
    states.set(ProtoEnv(Arc::new(ProtoEnvironment::new()?)));

    resources.set(Console::new(quiet));
}

#[system]
pub async fn load_workspace(states: StatesMut, resources: ResourcesMut) {
    let workspace = moon::load_workspace_from(
        Arc::clone(states.get::<ProtoEnv>()),
        Arc::new(resources.get::<Console>().to_owned()),
    )
    .await?;

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
    // TODO fix starbase
    // let configs = {
    //     resources
    //         .get::<Workspace>()
    //         .toolchain_config
    //         .tools
    //         .iter()
    //         .map(|(k, v)| (PluginId::raw(k), v.to_owned()))
    //         .collect::<FxHashMap<_, _>>()
    // };

    resources.set(ExtensionRegistry::new(
        Arc::clone(moon_env),
        Arc::clone(proto_env),
    ));

    resources.set(PlatformRegistry {
        configs: FxHashMap::default(),
        registry: PluginRegistry::new(
            PluginType::Platform,
            Arc::clone(moon_env),
            Arc::clone(proto_env),
        ),
    });
}

#[system]
pub async fn install_proto(
    proto_env: StateRef<ProtoEnv>,
    workspace: ResourceRef<Workspace>,
    console: ResourceRef<Console>,
) {
    let bin_name = exe_name("proto");
    let install_dir = proto_env
        .store
        .inventory_dir
        .join("proto")
        .join(PROTO_CLI_VERSION);

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

    console.out.print_checkpoint(
        Checkpoint::Setup,
        format!("installing proto {}", PROTO_CLI_VERSION),
    )?;

    // If offline but a primary proto binary exists,
    // use that instead of failing, even if a different version!
    if is_offline() {
        let existing_bin = proto_env.store.bin_dir.join(&bin_name);

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
        &proto_env.store.temp_dir,
        |_, _| {},
    )
    .await?;

    debug!("Unpacking archive and installing proto");

    unpack_release(result, &install_dir, &proto_env.store.temp_dir, false)?;

    debug!("Successfully installed proto!");
}
