#![allow(unused_variables)]

use moon_common::is_ci;
use notify_rust::{Notification, Timeout};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{debug, trace};

#[cfg(target_os = "macos")]
fn configure_application() {
    use notify_rust::{get_bundle_identifier_or_default, set_application};

    static APP_NAME: OnceLock<()> = OnceLock::new();

    APP_NAME.get_or_init(|| {
        let id = get_bundle_identifier_or_default("moon");

        // Finder is already the default
        if id != "com.apple.Finder" {
            if let Err(error) = set_application(&id) {
                debug!("Failed to set terminal source application: {error}");
            }
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn configure_application() {}

// https://docs.rs/notify-rust/latest/notify_rust/#platform-differences
pub fn notify_terminal(title: impl AsRef<str>, description: impl AsRef<str>) -> miette::Result<()> {
    if is_ci() {
        return Ok(());
    }

    configure_application();

    trace!("Sending terminal notification");

    match Notification::new()
        .appname("moon")
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
