use serde::{Deserialize, Serialize};
use starbase_utils::fs::{self, FsError};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Get the daemon endpoint string for this workspace.
///
/// - Unix: returns a socket file path like `/.moon/daemon/moond.sock`
/// - Windows: returns a named pipe like `\\.\pipe\moon-daemon-<hash>`
pub fn get_endpoint(daemon_dir: &Path) -> String {
    #[cfg(unix)]
    {
        get_sock_path(daemon_dir).to_string_lossy().into_owned()
    }

    #[cfg(windows)]
    {
        let hash = format!(
            "{:x}",
            md5::compute(daemon_dir.to_string_lossy().as_bytes())
        );

        format!(r"\\.\pipe\moon-daemon-{hash}")
    }
}

pub fn get_sock_path(daemon_dir: &Path) -> PathBuf {
    daemon_dir.join("moond.sock")
}

/// Path to the daemon ownership lock. The running daemon holds an exclusive
/// advisory lock on this file for its entire lifetime; whoever holds it owns
/// the endpoint files. Clients never probe it — liveness is a connection —
/// so a momentary probe can't race a starting daemon into exiting.
pub fn get_lock_path(daemon_dir: &Path) -> PathBuf {
    daemon_dir.join("daemon.lock")
}

/// Path to the spawn single-flight lock, held briefly while a process spawns
/// (or force-stops) a daemon so concurrent CLIs don't race to spawn.
pub fn get_spawn_lock_path(daemon_dir: &Path) -> PathBuf {
    daemon_dir.join("spawn.lock")
}

/// Path to the daemon state file. This is informational only — the pid and
/// version for display, and the pid as a kill fallback. Ownership is decided
/// by [`get_lock_path`], never by this file, so a stale copy left by a crash
/// is harmless.
pub fn get_state_path(daemon_dir: &Path) -> PathBuf {
    daemon_dir.join("daemon.json")
}

/// Informational record of the running daemon, written after it takes
/// ownership of the endpoint. Never used to decide liveness.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DaemonInfo {
    pub pid: u32,
    pub version: String,
    pub endpoint: String,
    /// Unix epoch milliseconds when the daemon started.
    pub started_at: u64,
}

impl DaemonInfo {
    pub fn new(pid: u32, version: String, endpoint: String) -> Self {
        Self {
            pid,
            version,
            endpoint,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis() as u64)
                .unwrap_or(0),
        }
    }
}

pub fn read_state(daemon_dir: &Path) -> Option<DaemonInfo> {
    let content = fs::read_file_with_lock(get_state_path(daemon_dir)).ok()?;

    serde_json::from_str(&content).ok()
}

pub fn write_state(daemon_dir: &Path, info: DaemonInfo) -> Result<(), FsError> {
    let content =
        serde_json::to_string_pretty(&info).expect("DaemonInfo serialization cannot fail");

    fs::write_file_with_lock(get_state_path(daemon_dir), content)
}

/// Remove the endpoint files owned by the daemon: the socket and the state
/// file. The lock files are left in place — they're empty, reused across
/// runs, and on Windows can't be removed while a handle holds them.
pub fn cleanup_daemon_files(daemon_dir: &Path) -> Result<(), FsError> {
    debug!(daemon_dir = ?daemon_dir, "Cleaning daemon files");

    fs::remove_file(get_state_path(daemon_dir))?;
    fs::remove_file(get_sock_path(daemon_dir))?;

    Ok(())
}
