use moon_api::Launchpad;
use moon_app_components::{AppConsole, MoonEnv};
use moon_common::{color, is_formatted_output, is_test_env};
use moon_console::Checkpoint;
use moon_workspace::Workspace;
use starbase::system;
use tracing::debug;

#[system]
pub async fn check_for_new_version(
    moon_env: StateRef<MoonEnv>,
    workspace: ResourceRef<Workspace>,
    console: ResourceRef<AppConsole>,
) {
    if is_test_env() || is_formatted_output() || !moon::is_telemetry_enabled() {
        return Ok(());
    }

    match Launchpad::check_version(&workspace.cache_engine, moon_env, false).await {
        Ok(Some(result)) => {
            if !result.update_available {
                return Ok(());
            }

            console.out.print_checkpoint(
                Checkpoint::Announcement,
                format!(
                    "There's a new version of moon available, {} (currently on {})!",
                    color::hash(result.remote_version.to_string()),
                    result.local_version,
                ),
            )?;

            if let Some(newer_message) = result.message {
                console
                    .out
                    .print_checkpoint(Checkpoint::Announcement, newer_message)?;
            }

            console.out.print_checkpoint(
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
    }
}
