use moon_common::is_ci;
use notify_rust::{Notification, Timeout};
use std::time::Duration;
use tracing::{debug, trace};

#[cfg(target_os = "macos")]
fn configure_application(_notification: &mut Notification) {
    use notify_rust::{get_bundle_identifier_or_default, set_application};
    use std::env;
    use std::sync::OnceLock;

    static APP_NAME: OnceLock<()> = OnceLock::new();

    APP_NAME.get_or_init(|| {
        // Try and detect the current terminal identifier so that
        // notifications come from it, and we can then use its OS settings
        let id = env::var("__CFBundleIdentifier").unwrap_or_else(|_| {
            get_bundle_identifier_or_default(
                env::var("TERM_PROGRAM")
                    .unwrap_or_else(|_| "moon".into())
                    .as_str(),
            )
        });

        // Finder is already the default
        if id != "com.apple.Finder"
            && let Err(error) = set_application(&id)
        {
            debug!("Failed to set terminal source application: {error}");
        }
    });
}

#[cfg(target_os = "linux")]
fn configure_application(_notification: &mut Notification) {}

#[cfg(windows)]
fn configure_application(notification: &mut Notification) {
    use std::env;
    use std::sync::OnceLock;
    use system_env::find_command_on_path;

    static APP_ID: OnceLock<bool> = OnceLock::new();

    // Try and use Windows Terminal as the app ID,
    // otherwise this will fallback to legacy PowerShell
    if env::var("WT_SESSION").is_ok()
        || *APP_ID.get_or_init(|| find_command_on_path("wt").is_some())
    {
        notification.app_id("Microsoft.WindowsTerminal_8wekyb3d8bbwe!App");
    }
}

// https://docs.rs/notify-rust/latest/notify_rust/#platform-differences
pub fn notify_terminal(title: impl AsRef<str>, description: impl AsRef<str>) -> miette::Result<()> {
    if is_ci() {
        return Ok(());
    }

    let mut notification = Notification::new();

    notification
        .appname("moon")
        .summary(title.as_ref())
        .body(description.as_ref())
        .timeout(Timeout::from(Duration::from_secs(10)));

    configure_application(&mut notification);

    trace!("Sending terminal notification");

    match notification.show() {
        Ok(_) => {
            trace!("Sent terminal notification");
        }
        Err(error) => {
            debug!("Failed to send terminal notification: {error}");
        }
    };

    Ok(())
}
