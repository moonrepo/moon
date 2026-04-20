use crate::commands::upgrade::{InstalledWith, is_installed_with};
use crate::session::MoonSession;
use moon_api::Launchpad;
use moon_common::{color, is_formatted_output, is_test_env};
use moon_console::Checkpoint;
use starbase::AppResult;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn check_for_new_version(session: &MoonSession, manifest_url: &str) -> AppResult {
    if is_test_env() || is_formatted_output() {
        return Ok(None);
    }

    let Some(launchpad) = Launchpad::instance() else {
        return Ok(None);
    };

    let console = &session.console;
    let cache_engine = session.get_cache_engine()?;

    match launchpad
        .check_version(&cache_engine, false, manifest_url)
        .await
    {
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

            if let Some(message) = result.message {
                console.print_checkpoint(Checkpoint::Announcement, message)?;
            }

            if let Some(url) = result.url {
                console.print_checkpoint(Checkpoint::Announcement, format!("Learn more: {url}"))?;
            }

            match is_installed_with(session)? {
                InstalledWith::Proto => {
                    console.print_checkpoint(
                        Checkpoint::Announcement,
                        format!(
                            "Run {} to upgrade!",
                            color::shell(format!(
                                "proto install moon {} --pin",
                                result.remote_version
                            ))
                        ),
                    )?;
                }
                InstalledWith::Moon => {
                    console.print_checkpoint(
                        Checkpoint::Announcement,
                        format!("Run {} to get started!", color::shell("moon upgrade")),
                    )?;
                }
                InstalledWith::Unknown(_) => {
                    console.print_checkpoint(
                        Checkpoint::Announcement,
                        format!(
                            "Install with: {}",
                            color::url("https://moonrepo.dev/docs/install"),
                        ),
                    )?;
                }
            };
        }
        Err(error) => {
            debug!("Failed to check for current version: {}", error);
        }
        _ => {}
    };

    Ok(None)
}
