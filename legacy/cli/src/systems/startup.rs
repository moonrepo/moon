use crate::app::GlobalArgs;
use moon_app_components::{Console, ExtensionRegistry, MoonEnv, ProtoEnv, WorkspaceRoot};
use moon_common::{consts::PROTO_CLI_VERSION, is_test_env, path::exe_name};
use moon_console::Checkpoint;
use moon_console_reporter::DefaultReporter;
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

// #[system]
// pub async fn load_environments(states: States) {
pub async fn load_environments(
    states: starbase::States,
    resources: starbase::Resources,
    emitters: starbase::Emitters,
) -> starbase::SystemResult {
    states.set(MoonEnv(Arc::new(MoonEnvironment::new()?))).await;

    states
        .set(ProtoEnv(Arc::new(ProtoEnvironment::new()?)))
        .await;

    // test

    Ok(())
}

// #[system]
// pub async fn load_workspace(states: States, resources: Resources) {
pub async fn load_workspace(
    states: starbase::States,
    resources: starbase::Resources,
    emitters: starbase::Emitters,
) -> starbase::SystemResult {
    let console = {
        let quiet = states.get::<GlobalArgs>().await.quiet;

        let mut console = Console::new(quiet);
        console.set_reporter(DefaultReporter::default());
        console
    };

    let workspace = {
        let proto_env = states.get::<ProtoEnv>().await;

        moon::load_workspace_from(Arc::clone(&proto_env), Arc::new(console.clone())).await?
    };

    // Ensure our env instance is using the found workspace root,
    // as this is required for plugins to function entirely!
    {
        let mut moon_env = states.get::<MoonEnv>().await;

        Arc::get_mut(&mut moon_env).unwrap().workspace_root = workspace.root.clone();
    }

    states.set(WorkspaceRoot(workspace.root.clone())).await;

    resources.set(console).await;
    resources.set(workspace).await;

    // test
    Ok(())
}

// #[system]
// pub async fn create_plugin_registries(
//     resources: Resources,
//     moon_env: StateRef<MoonEnv>,
//     proto_env: StateRef<ProtoEnv>,
// ) {
pub async fn create_plugin_registries(
    states: starbase::States,
    resources: starbase::Resources,
    emitters: starbase::Emitters,
) -> starbase::SystemResult {
    let moon_env_base = states.get::<MoonEnv>().await;
    let moon_env = moon_env_base.read();
    let proto_env_base = states.get::<ProtoEnv>().await;
    let proto_env = proto_env_base.read();

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

    resources
        .set(ExtensionRegistry::new(
            Arc::clone(moon_env),
            Arc::clone(proto_env),
        ))
        .await;

    resources
        .set(PlatformRegistry {
            configs: FxHashMap::default(),
            registry: PluginRegistry::new(
                PluginType::Platform,
                Arc::clone(moon_env),
                Arc::clone(proto_env),
            ),
        })
        .await;

    // test

    Ok(())
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
    env::set_var("PROTO_LOOKUP_DIR", &install_dir);

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
