use semver::Version;
use starbase_styles::color;
use std::env;
use std::path::PathBuf;
use tracing::{debug, instrument};

#[derive(Debug)]
pub struct AppInfo {
    /// The executed moon binary that kicked off the app process.
    pub current_exe: Option<PathBuf>,

    /// Is running with a global moon binary.
    pub global: bool,

    /// The moon binary that is currently running. This may be different
    /// than `current_exe` if we detect a local binary to use instead of
    /// the running global.
    pub running_exe: Option<PathBuf>,

    /// Current version of moon.
    pub version: Version,
}

/// Detect important information about the currently running moon process.
#[instrument]
pub fn detect_app_process_info() -> AppInfo {
    let current_exe = env::current_exe().ok();
    let version = env!("CARGO_PKG_VERSION");

    // TODO args
    if let Some(exe) = &current_exe {
        debug!("Running moon v{} (with {})", version, color::path(exe),);
    } else {
        debug!("Running moon v{}", version);
    }

    env::set_var("MOON_VERSION", version);

    AppInfo {
        running_exe: current_exe.clone(),
        current_exe,
        global: false,
        version: Version::parse(version).unwrap(),
    }
}
