use std::time::Duration;

use crate::app::Commands;
use crate::states::CurrentCommand;
use moon_api::Launchpad;
use moon_cache::CacheEngine;
use moon_common::{color, is_test_env, is_unformatted_stdout};
use moon_terminal::{get_checkpoint_prefix, Checkpoint};
use moon_utils::get_workspace_root;
use starbase::system;
use tokio::time::sleep;
use tracing::debug;

#[system]
pub async fn check_for_new_version(global_args: StateRef<CurrentCommand>) {
    if matches!(
        &global_args.command,
        Commands::Check { .. } | Commands::Ci { .. } | Commands::Run { .. } | Commands::Sync { .. }
    ) {
        // Wait for telemetry to be enabled!
        sleep(Duration::from_millis(150)).await;

        if is_test_env() || !is_unformatted_stdout() || !moon::is_telemetry_enabled() {
            return Ok(());
        }

        let current_version = env!("CARGO_PKG_VERSION");
        let prefix = get_checkpoint_prefix(Checkpoint::Announcement);
        let cache_engine = CacheEngine::new(&get_workspace_root())?;

        match Launchpad::check_version(&cache_engine, current_version, false).await {
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
}
