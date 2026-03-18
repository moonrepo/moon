use starbase_utils::fs::{self, FsError};
use std::path::{Path, PathBuf};
use tracing::trace;

pub fn get_daemon_dir(cache_dir: &Path) -> PathBuf {
    cache_dir.join("daemon")
}

/// Get the daemon endpoint string for this workspace.
///
/// - Unix: returns a socket file path like `~/.moon/daemon/moond.sock`
/// - Windows: returns a named pipe like `\\.\pipe\moon-daemon-<hash>`
#[allow(unused_variables)]
pub fn get_endpoint(workspace_root: &Path, cache_dir: &Path) -> String {
    #[cfg(unix)]
    {
        get_daemon_dir(cache_dir)
            .join("moond.sock")
            .to_string_lossy()
            .into_owned()
    }

    #[cfg(windows)]
    {
        let hash = format!(
            "{:x}",
            md5::compute(workspace_root.to_string_lossy().as_bytes())
        );

        format!(r"\\.\pipe\moon-daemon-{hash}")
    }
}

pub fn get_pid_path(cache_dir: &Path) -> PathBuf {
    get_daemon_dir(cache_dir).join("moond.pid")
}

pub fn read_pid(pid_path: &Path) -> Option<u32> {
    let content = fs::read_file(pid_path).ok()?;
    content.trim().parse().ok()
}

pub fn write_pid(pid_path: &Path, pid: u32) -> Result<(), FsError> {
    fs::write_file(pid_path, pid.to_string())
}

pub fn cleanup_daemon_files(workspace_root: &Path, cache_dir: &Path) -> Result<(), FsError> {
    let pid_path = get_pid_path(cache_dir);

    fs::remove_file(&pid_path)?;

    trace!(pid_file = ?pid_path, "Cleaned up daemon pid file");

    #[cfg(unix)]
    {
        let endpoint = get_endpoint(workspace_root, cache_dir);

        fs::remove_file(&endpoint)?;

        trace!(socket = endpoint, "Cleaned up daemon socket file");
    }

    Ok(())
}

#[cfg(unix)]
pub fn is_process_alive(pid: u32) -> bool {
    // Reject invalid PIDs (0 = all processes in group, negative = process groups)
    let pid = pid as libc::pid_t;

    if pid <= 0 {
        return false;
    }

    // Signal 0 doesn't send a signal but checks process existence
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(windows)]
pub fn is_process_alive(pid: u32) -> bool {
    use std::os::windows::io::FromRawHandle;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    unsafe {
        let handle = windows_sys::Win32::System::Threading::OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        );

        if handle.is_null() {
            return false;
        }

        let mut exit_code: u32 = 0;
        let result =
            windows_sys::Win32::System::Threading::GetExitCodeProcess(handle, &mut exit_code);

        windows_sys::Win32::Foundation::CloseHandle(handle);

        result != 0 && exit_code == STILL_ACTIVE
    }
}

pub fn is_daemon_running(cache_dir: &Path) -> bool {
    let pid_path = get_pid_path(cache_dir);

    match read_pid(&pid_path) {
        Some(pid) => is_process_alive(pid),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_pid_path() {
        let store = Path::new("/home/user/.moon");
        let pid_path = get_pid_path(store);
        assert!(pid_path.starts_with("/home/user/.moon/daemon/"));
        assert!(pid_path.to_string_lossy().ends_with(".pid"));
    }

    #[cfg(unix)]
    #[test]
    fn test_get_endpoint_unix() {
        let store = Path::new("/home/user/.moon");
        let workspace = Path::new("/home/user/project");
        let endpoint = get_endpoint(workspace, store);
        assert!(endpoint.starts_with("/home/user/.moon/daemon/"));
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

    #[test]
    fn test_is_process_alive_current() {
        let pid = std::process::id();
        assert!(is_process_alive(pid));
    }

    #[test]
    fn test_is_process_alive_dead() {
        // u32::MAX wraps to -1 as pid_t, which is a special value for kill().
        // Use a very high but valid PID that is extremely unlikely to exist.
        assert!(!is_process_alive(4_000_000));
    }
}
