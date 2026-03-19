use starbase_utils::fs::{self, FsError};
use std::path::{Path, PathBuf};
use tracing::trace;

/// Get the daemon endpoint string for this workspace.
///
/// - Unix: returns a socket file path like `/.moon/daemon/moond.sock`
/// - Windows: returns a named pipe like `\\.\pipe\moon-daemon-<hash>`
pub fn get_endpoint(daemon_dir: &Path) -> String {
    #[cfg(unix)]
    {
        daemon_dir.join("moond.sock").to_string_lossy().into_owned()
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

pub fn get_pid_path(daemon_dir: &Path) -> PathBuf {
    daemon_dir.join("moond.pid")
}

pub fn read_pid(pid_path: &Path) -> Option<u32> {
    let content = fs::read_file_with_lock(pid_path).ok()?;
    content.trim().parse().ok()
}

pub fn write_pid(pid_path: &Path, pid: u32) -> Result<(), FsError> {
    fs::write_file_with_lock(pid_path, pid.to_string())
}

pub fn cleanup_daemon_files(daemon_dir: &Path) -> Result<(), FsError> {
    trace!(daemon_dir = ?daemon_dir, "Cleaning daemon files");

    fs::remove_dir_all(daemon_dir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_pid_path() {
        let store = Path::new("/home/user/.moon/daemon");
        let pid_path = get_pid_path(store);
        assert!(pid_path.starts_with("/home/user/.moon/daemon"));
        assert!(pid_path.to_string_lossy().ends_with(".pid"));
    }

    #[cfg(unix)]
    #[test]
    fn test_get_endpoint_unix() {
        let store = Path::new("/home/user/.moon/daemon");
        let endpoint = get_endpoint(store);
        assert!(endpoint.starts_with("/home/user/.moon/daemon"));
        assert!(endpoint.ends_with(".sock"));
    }

    #[test]
    fn test_read_write_pid() {
        let dir = std::env::temp_dir().join("moon_daemon_test_pid");
        let _ = fs::create_dir_all(&dir);
        let pid_path = dir.join("test.pid");

        write_pid(&pid_path, 12345).unwrap();
        assert_eq!(read_pid(&pid_path), Some(12345));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_read_pid_missing_file() {
        let path = Path::new("/nonexistent/path/test.pid");
        assert_eq!(read_pid(path), None);
    }
}
