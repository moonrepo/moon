#![allow(unused_variables)]

use moon_common::is_ci;
use notify_rust::{Notification, Timeout};
use std::env;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{debug, trace};

static APP_NAME: OnceLock<Option<String>> = OnceLock::new();

#[cfg(target_os = "macos")]
fn configure_application() -> Option<&'static str> {
    use notify_rust::{get_bundle_identifier_or_default, set_application};

    APP_NAME
        .get_or_init(|| {
            // Try and detect the current terminal identifier so that
            // notifications come from it, and we can use its OS settings
            let id = env::var("__CFBundleIdentifier").unwrap_or_else(|_| {
                get_bundle_identifier_or_default(
                    env::var("TERM_PROGRAM")
                        .unwrap_or_else(|_| "moon".into())
                        .as_str(),
                )
            });

            // Finder is already the default
            if id != "com.apple.Finder" {
                if let Err(error) = set_application(&id) {
                    debug!("Failed to set terminal source application: {error}");
                }
            }

            // App name is not used by macOS
            None
        })
        .as_deref()
}

#[cfg(not(target_os = "macos"))]
fn configure_application() -> Option<&'static str> {
    None
}

// https://docs.rs/notify-rust/latest/notify_rust/#platform-differences
pub fn notify_terminal(title: impl AsRef<str>, description: impl AsRef<str>) -> miette::Result<()> {
    if is_ci() {
        return Ok(());
    }

    let name = configure_application();

    trace!("Sending terminal notification");

    match Notification::new()
        .appname(name.unwrap_or("moon"))
        .summary(title.as_ref())
        .body(description.as_ref())
        .timeout(Timeout::from(Duration::from_secs(10)))
        .show()
    {
        Ok(handle) => {
            trace!("Sent terminal notification");

            #[cfg(all(unix, not(target_os = "macos")))]
            {
                handle.on_close(|reason| trace!("Closed terminal notification: {:?}", reason));
            }
        }
        Err(error) => {
            debug!("Failed to send terminal notification: {error}");
        }
    };

    Ok(())
}
