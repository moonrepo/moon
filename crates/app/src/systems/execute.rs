use moon_api::Launchpad;
use moon_cache::CacheEngine;
use moon_common::{color, is_formatted_output, is_test_env};
use moon_console::{Checkpoint, Console};
use moon_env::MoonEnvironment;
use starbase::AppResult;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn check_for_new_version(
    console: &Console,
    moon_env: &MoonEnvironment,
    cache_engine: &CacheEngine,
    manifest_url: &str,
) -> AppResult {
    if is_test_env() || is_formatted_output() {
        return Ok(None);
    }

    match Launchpad::check_version(cache_engine, moon_env, false, manifest_url).await {
        Ok(Some(result)) => {
            if !result.update_available {
                return Ok(None);
            }

            console.print_checkpoint(
                Checkpoint::Announcement,
                format!(
                    "There's a new version of moon available, {} (currently on {})!",
                    color::hash(result.remote_version.to_string()),
                    result.local_version,
                ),
            )?;

            if let Some(newer_message) = result.message {
                console.print_checkpoint(Checkpoint::Announcement, newer_message)?;
            }

            console.print_checkpoint(
                Checkpoint::Announcement,
                format!(
                    "Run {} or install from {}",
                    color::success("moon upgrade"),
                    color::url("https://moonrepo.dev/docs/install"),
                ),
            )?;
        }
        Err(error) => {
            debug!("Failed to check for current version: {}", error);
        }
        _ => {}
    };

    Ok(None)
}
