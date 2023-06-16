use starbase::Resource;
use std::path::PathBuf;

#[derive(Debug, Resource)]
pub struct AppInfo {
    /// The executed moon binary that kicked off the app process.
    pub current_exe: Option<PathBuf>,

    /// Is running with a global moon binary.
    pub global: bool,

    /// The moon binary that is currently running. This may be different
    /// than `current_exe` if we detect a local binary to use instead of
    /// the running global.
    pub running_exe: Option<PathBuf>,

    /// Current versio of moon.
    pub version: String,
}
