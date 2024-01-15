use crate::app::{App as CLI, Commands};
use moon_api::Launchpad;
use moon_app_components::{ExtensionRegistry, MoonDir, WorkingDir, WorkspaceRoot};
use moon_common::{
    color, consts::PROTO_CLI_VERSION, get_moon_dir, is_test_env, is_unformatted_stdout,
    path::exe_name,
};
use moon_terminal::{get_checkpoint_prefix, print_checkpoint, Checkpoint};
use moon_workspace::Workspace;
use proto_core::{is_offline, ProtoError};
use proto_installer::{determine_triple, download_release, unpack_release};
use starbase::system;
use std::env;
use tracing::debug;

pub fn requires_workspace(cli: &CLI) -> bool {
    !matches!(
        cli.command,
        Commands::Completions(_) | Commands::Init(_) | Commands::Setup | Commands::Upgrade
    )
}

pub fn requires_toolchain(cli: &CLI) -> bool {
    matches!(
        cli.command,
        Commands::Bin(_) | Commands::Docker { .. } | Commands::Node { .. } | Commands::Teardown
    )
}

#[system]
pub async fn create_components(states: StatesMut, resources: ResourcesMut) {
    let moon_dir = get_moon_dir();
    let plugins_dir = moon_dir.join("plugins");
    let temp_dir = moon_dir.join("temp");

    resources.set(ExtensionRegistry::new(
        &plugins_dir.join("extensions"),
        &temp_dir,
    ));

    states.set(MoonDir(moon_dir));
}

#[system]
pub async fn load_workspace(states: StatesMut, resources: ResourcesMut) {
    let workspace = moon::load_workspace().await?;

    states.set(WorkingDir(workspace.working_dir.clone()));
    states.set(WorkspaceRoot(workspace.root.clone()));

    resources.set(workspace);
}

#[system]
pub async fn load_toolchain() {
    moon::load_toolchain().await?;
}

#[system]
pub async fn check_for_new_version(workspace: ResourceRef<Workspace>) {
    if is_test_env() || !is_unformatted_stdout() || !moon::is_telemetry_enabled() {
        return Ok(());
    }

    let prefix = get_checkpoint_prefix(Checkpoint::Announcement);

    match Launchpad::check_version(&workspace.cache_engine, env!("CARGO_PKG_VERSION"), false).await
    {
        Ok(Some(result)) => {
            if !result.update_available {
                return Ok(());
            }

            println!(
                "{} There's a new version of moon available, {} (currently on {})!",
                prefix,
                color::hash(result.remote_version.to_string()),
                result.local_version,
            );

            if let Some(newer_message) = result.message {
                println!("{} {}", prefix, newer_message);
            }

            println!(
                "{} Run {} or install from {}",
                prefix,
                color::success("moon upgrade"),
                color::url("https://moonrepo.dev/docs/install"),
            );
        }
        Err(error) => {
            debug!("Failed to check for current version: {}", error);
        }
        _ => {}
    }
}

#[system]
pub async fn install_proto(workspace: ResourceRef<Workspace>) {
    let bin_name = exe_name("proto");
    let install_dir = workspace
        .proto_env
        .tools_dir
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

    if is_unformatted_stdout() {
        print_checkpoint(
            format!("installing proto {}", PROTO_CLI_VERSION),
            Checkpoint::Setup,
        );
    }

    // If offline but a primary proto binary exists,
    // use that instead of failing, even if a different version!
    if is_offline() {
        let existing_bin = workspace.proto_env.bin_dir.join(&bin_name);

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
        &workspace.proto_env.temp_dir,
        |_, _| {},
    )
    .await?;

    debug!("Unpacking archive and installing proto");

    unpack_release(result, &install_dir, &workspace.proto_env.temp_dir)?;

    debug!("Successfully installed proto!");
}
