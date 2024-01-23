use moon_api::Launchpad;
use moon_app_components::MoonEnv;
use moon_common::{color, is_test_env, is_unformatted_stdout};
use moon_terminal::{get_checkpoint_prefix, Checkpoint};
use moon_workspace::Workspace;
use starbase::system;
use tracing::debug;

#[system]
pub async fn check_for_new_version(moon_env: StateRef<MoonEnv>, workspace: ResourceRef<Workspace>) {
    if is_test_env() || !is_unformatted_stdout() || !moon::is_telemetry_enabled() {
        return Ok(());
    }

    let prefix = get_checkpoint_prefix(Checkpoint::Announcement);

    match Launchpad::check_version(&workspace.cache_engine, moon_env, false).await {
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
