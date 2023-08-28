use crate::app::{App as CLI, Commands};
use moon_api::Launchpad;
use moon_common::{color, is_test_env, is_unformatted_stdout};
use moon_terminal::{get_checkpoint_prefix, Checkpoint};
use moon_workspace::Workspace;
use starbase::system;
use tracing::debug;

#[system]
pub async fn load_workspace(cli: StateRef<CLI>, resources: ResourcesMut) {
    match &cli.command {
        Commands::Completions(_) | Commands::Init(_) | Commands::Setup => {
            // Do nothing
        }
        Commands::Bin(_) | Commands::Docker { .. } | Commands::Node { .. } | Commands::Teardown => {
            resources.set(moon::load_workspace_with_toolchain().await?);
        }
        _ => {
            resources.set(moon::load_workspace().await?);
        }
    };
}

#[system]
pub async fn check_for_new_version(workspace: ResourceRef<Workspace>) {
    if is_test_env() || !is_unformatted_stdout() || !moon::is_telemetry_enabled() {
        return Ok(());
    }

    let current_version = env!("CARGO_PKG_VERSION");
    let prefix = get_checkpoint_prefix(Checkpoint::Announcement);

    match Launchpad::check_version(&workspace.cache_engine, current_version, false).await {
        Ok(Some(latest)) => {
            println!(
                "{} There's a new version of moon available, {} (currently on {})!",
                prefix,
                color::success(latest.current_version),
                current_version,
            );

            if let Some(newer_message) = latest.message {
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
