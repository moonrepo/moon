use crate::client::DaemonClient;
use crate::daemon_error::DaemonError;
use crate::endpoint::*;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, trace, warn};

/// Maximum time to wait for the daemon to become ready after spawning.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Interval between readiness polls.
const POLL_INTERVAL: Duration = Duration::from_millis(150);

/// Maximum time to wait for the daemon to shut down gracefully before killing.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Spawn the daemon as a detached background process.
///
/// The daemon is started by re-executing the current binary with the
/// `daemon-server` internal argument, which causes the process to run
/// `start_daemon_server` and block until shutdown.
///
/// On Unix the child is placed in a new session (`setsid` equivalent)
/// so it survives the parent exiting.  On Windows it is created as a
/// detached process.
///
/// Returns the PID of the spawned child once it is alive.
pub async fn spawn_daemon(
    workspace_root: &Path,
    cache_dir: &Path,
    moon_version: &str,
) -> miette::Result<u32> {
    // If already running, return its PID.
    if let Some(pid) = check_running(cache_dir) {
        debug!(pid, "Daemon already running, skipping spawn");
        return Err(DaemonError::AlreadyRunning { pid }.into());
    }

    // Clean up stale files from a previous crashed daemon.
    cleanup_stale_state(workspace_root, cache_dir);

    let exe = std::env::current_exe().map_err(|error| DaemonError::StartFailed {
        error: Box::new(error),
    })?;

    debug!(binary = ?exe, "Spawning daemon process");

    let child = spawn_detached(
        &exe,
        &[
            "daemon-server",
            "--workspace-root",
            &workspace_root.to_string_lossy(),
            "--cache-dir",
            &cache_dir.to_string_lossy(),
            "--moon-version",
            moon_version,
        ],
    )?;

    let pid = child.id();

    debug!(pid, "Daemon process spawned, waiting for readiness");

    wait_for_ready(cache_dir, pid).await?;

    Ok(pid)
}

/// Ensure the daemon is running and return a connected client.
///
/// If the daemon is not running it will be spawned first.  This is the
/// primary entry point for CLI commands that need the daemon.
pub async fn ensure_daemon_running(
    workspace_root: &Path,
    cache_dir: &Path,
    moon_version: &str,
) -> miette::Result<DaemonClient> {
    // Fast path: daemon is already running.
    if is_daemon_running(cache_dir) {
        debug!("Daemon already running, connecting");
        return DaemonClient::connect(workspace_root, cache_dir).await;
    }

    // Spawn and then connect.
    spawn_daemon(workspace_root, cache_dir, moon_version).await?;

    DaemonClient::connect(workspace_root, cache_dir).await
}

/// Gracefully stop the daemon via RPC, falling back to a process kill.
///
/// Returns `true` if the daemon was stopped, `false` if it was not running.
pub async fn stop_daemon(workspace_root: &Path, cache_dir: &Path) -> miette::Result<bool> {
    let pid = match check_running(cache_dir) {
        Some(pid) => pid,
        None => {
            debug!("Daemon not running, nothing to stop");
            return Ok(false);
        }
    };

    debug!(pid, "Stopping daemon");

    // Try graceful shutdown via RPC first.
    match graceful_shutdown(workspace_root, cache_dir).await {
        Ok(()) => {
            debug!(pid, "Daemon stopped gracefully");
            return Ok(true);
        }
        Err(error) => {
            warn!(pid, ?error, "Graceful shutdown failed, falling back to kill");
        }
    }

    // Forcefully kill the process.
    kill_process(pid)?;

    // Clean up stale files since the server won't clean up after itself.
    let _ = cleanup_daemon_files(workspace_root, cache_dir);

    debug!(pid, "Daemon killed forcefully");

    Ok(true)
}

/// Check if the daemon process is alive and return its PID.
fn check_running(cache_dir: &Path) -> Option<u32> {
    let pid_path = get_pid_path(cache_dir);
    let pid = read_pid(&pid_path)?;

    if is_process_alive(pid) {
        Some(pid)
    } else {
        None
    }
}

/// Remove stale PID / socket files left behind by a crashed daemon.
fn cleanup_stale_state(workspace_root: &Path, cache_dir: &Path) {
    let pid_path = get_pid_path(cache_dir);

    if pid_path.exists() {
        trace!("Cleaning up stale daemon files");
        let _ = cleanup_daemon_files(workspace_root, cache_dir);
    }
}

/// Attempt a graceful shutdown via the `Stop` RPC.
async fn graceful_shutdown(workspace_root: &Path, cache_dir: &Path) -> miette::Result<()> {
    let mut client = DaemonClient::connect(workspace_root, cache_dir).await?;

    client.stop().await?;

    // Wait for the process to actually exit.
    let pid_path = get_pid_path(cache_dir);
    let deadline = tokio::time::Instant::now() + SHUTDOWN_TIMEOUT;

    while tokio::time::Instant::now() < deadline {
        match read_pid(&pid_path) {
            Some(pid) if is_process_alive(pid) => {
                sleep(POLL_INTERVAL).await;
            }
            _ => return Ok(()),
        }
    }

    Err(DaemonError::StopTimedOut.into())
}

/// Wait for the daemon to write its PID file and become connectable.
async fn wait_for_ready(cache_dir: &Path, expected_pid: u32) -> miette::Result<()> {
    let pid_path = get_pid_path(cache_dir);
    let deadline = tokio::time::Instant::now() + STARTUP_TIMEOUT;

    while tokio::time::Instant::now() < deadline {
        // Check that the child is still alive.
        if !is_process_alive(expected_pid) {
            return Err(DaemonError::StartFailed {
                error: Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Daemon process exited unexpectedly during startup",
                )),
            }
            .into());
        }

        // Check for the PID file written by the server.
        if let Some(pid) = read_pid(&pid_path) {
            if pid == expected_pid {
                trace!(pid, "Daemon PID file detected, daemon is ready");
                return Ok(());
            }
        }

        sleep(POLL_INTERVAL).await;
    }

    Err(DaemonError::StartTimedOut.into())
}

// ── Platform-specific helpers ───────────────────────────────────────────

/// Spawn a fully detached child process.
#[cfg(unix)]
fn spawn_detached(
    exe: &std::path::Path,
    args: &[&str],
) -> miette::Result<std::process::Child> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let child = unsafe {
        Command::new(exe)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // Create a new session so the daemon survives the parent exiting.
            .pre_exec(|| {
                libc::setsid();
                Ok(())
            })
            .spawn()
    }
    .map_err(|error| DaemonError::StartFailed {
        error: Box::new(error),
    })?;

    Ok(child)
}

/// Spawn a fully detached child process (Windows).
#[cfg(windows)]
fn spawn_detached(
    exe: &std::path::Path,
    args: &[&str],
) -> miette::Result<std::process::Child> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
    const DETACH_FLAGS: u32 = 0x0000_0008 | 0x0000_0200;

    let child = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACH_FLAGS)
        .spawn()
        .map_err(|error| DaemonError::StartFailed {
            error: Box::new(error),
        })?;

    Ok(child)
}

/// Forcefully kill a process by PID.
#[cfg(unix)]
fn kill_process(pid: u32) -> miette::Result<()> {
    let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };

    if ret != 0 {
        let error = std::io::Error::last_os_error();

        // ESRCH means the process is already gone — not an error.
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(DaemonError::StopFailed {
                error: Box::new(error),
            }
            .into());
        }
    }

    Ok(())
}

/// Forcefully kill a process by PID (Windows).
#[cfg(windows)]
fn kill_process(pid: u32) -> miette::Result<()> {
    const PROCESS_TERMINATE: u32 = 0x0001;

    unsafe {
        let handle =
            windows_sys::Win32::System::Threading::OpenProcess(PROCESS_TERMINATE, 0, pid);

        if handle.is_null() {
            let error = std::io::Error::last_os_error();
            // If the process is already gone, that's fine.
            if error.raw_os_error() != Some(87) {
                return Err(DaemonError::StopFailed {
                    error: Box::new(error),
                }
                .into());
            }
            return Ok(());
        }

        let terminated =
            windows_sys::Win32::System::Threading::TerminateProcess(handle, 1);

        windows_sys::Win32::Foundation::CloseHandle(handle);

        if terminated == 0 {
            return Err(DaemonError::StopFailed {
                error: Box::new(std::io::Error::last_os_error()),
            }
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_check_running_no_pid_file() {
        let cache_dir = Path::new("/nonexistent/cache");
        assert!(check_running(cache_dir).is_none());
    }

    #[test]
    fn test_cleanup_stale_state_no_files() {
        // Should not panic when there are no files to clean up.
        let workspace = Path::new("/nonexistent/workspace");
        let cache_dir = Path::new("/nonexistent/cache");
        cleanup_stale_state(workspace, cache_dir);
    }
}
