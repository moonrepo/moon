use crate::daemon_error::DaemonError;
use moon_daemon_client::DaemonClient;
use moon_daemon_utils::{endpoint::*, sys::*};
use std::io::Error;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::{Instant, sleep};
use tracing::{debug, instrument, trace, warn};

/// Maximum time to wait for the daemon to become ready after spawning.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Interval between readiness polls.
const POLL_INTERVAL: Duration = Duration::from_millis(150);

/// Maximum time to wait for the daemon to shut down gracefully before killing.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub struct DaemonConnector {
    pub daemon_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl DaemonConnector {
    pub fn new(daemon_dir: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            daemon_dir,
            workspace_root,
        }
    }

    #[instrument(skip(self))]
    pub async fn connect(&self) -> miette::Result<DaemonClient> {
        // Ensure the server is running
        self.start_daemon().await?;

        DaemonClient::connect(&self.daemon_dir).await
    }

    pub fn get_log_file(&self) -> PathBuf {
        self.daemon_dir.join("server.log")
    }

    pub fn get_pid_file(&self) -> PathBuf {
        get_pid_path(&self.daemon_dir)
    }

    #[instrument(skip(self))]
    pub fn is_running(&self) -> Option<u32> {
        let pid_path = get_pid_path(&self.daemon_dir);

        if !pid_path.exists() {
            return None;
        }

        let pid = read_pid(&pid_path)?;

        if is_process_alive(pid) {
            Some(pid)
        } else {
            None
        }
    }

    #[instrument(skip(self))]
    pub async fn start_daemon(&self) -> miette::Result<u32> {
        // If already running, return its PID
        if let Some(pid) = self.is_running() {
            debug!(pid, "Daemon already running, skipping spawn");

            return Ok(pid);
        }

        // Clean up stale files from a previous crashed daemon
        cleanup_daemon_files(&self.daemon_dir)?;

        let exe_path = std::env::current_exe().map_err(|error| DaemonError::StartFailed {
            error: Box::new(error),
        })?;

        debug!(exe = ?exe_path, "Spawning daemon process");

        let mut command = create_detached_command(&exe_path);

        command
            .args(["daemon", "server", "--log", "trace", "--log-file"])
            .arg(self.get_log_file())
            .env("MOON_DAEMON_RUNNING", "true")
            .current_dir(&self.workspace_root);

        let child = command.spawn().map_err(|error| DaemonError::StartFailed {
            error: Box::new(error),
        })?;

        let pid = child.id();

        debug!(pid, "Daemon process spawned, waiting for readiness");

        self.wait_for_ready(pid).await?;

        debug!(pid, "Daemon is ready");

        Ok(pid)
    }

    #[instrument(skip(self))]
    pub async fn stop_daemon(&self) -> miette::Result<bool> {
        let pid = match self.is_running() {
            Some(pid) => pid,
            None => {
                debug!("Daemon not running, nothing to stop");

                return Ok(false);
            }
        };

        debug!(pid, "Stopping daemon");

        // Try graceful shutdown via RPC first
        match self.graceful_shutdown().await {
            Ok(()) => {
                debug!(pid, "Daemon stopped gracefully");

                return Ok(true);
            }
            Err(error) => {
                warn!(
                    pid,
                    ?error,
                    "Graceful shutdown failed, falling back to kill"
                );
            }
        };

        // Forcefully kill the process
        kill_process(pid).map_err(|error| DaemonError::StopFailed {
            error: Box::new(error),
        })?;

        // Clean up stale files since the server won't clean up after itself
        cleanup_daemon_files(&self.daemon_dir)?;

        debug!(pid, "Daemon killed forcefully");

        Ok(true)
    }

    #[instrument(skip(self))]
    async fn graceful_shutdown(&self) -> miette::Result<()> {
        let mut client = DaemonClient::connect(&self.daemon_dir).await?;
        client.stop().await?;

        // Wait for the process to actually exit
        let pid_path = get_pid_path(&self.daemon_dir);
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;

        while Instant::now() < deadline {
            match read_pid(&pid_path) {
                Some(pid) if is_process_alive(pid) => {
                    sleep(POLL_INTERVAL).await;
                }
                _ => return Ok(()),
            }
        }

        Err(DaemonError::StopTimedOut.into())
    }

    #[instrument(skip(self))]
    async fn wait_for_ready(&self, expected_pid: u32) -> miette::Result<()> {
        let pid_path = get_pid_path(&self.daemon_dir);
        let deadline = Instant::now() + STARTUP_TIMEOUT;

        while Instant::now() < deadline {
            // Check that the child is still alive
            if !is_process_alive(expected_pid) {
                return Err(DaemonError::StartFailed {
                    error: Box::new(Error::other(
                        "Daemon process exited unexpectedly during startup",
                    )),
                }
                .into());
            }

            // Check for the PID file written by the server
            if let Some(pid) = read_pid(&pid_path)
                && pid == expected_pid
            {
                trace!(pid, "Daemon PID file detected, daemon is ready");

                return Ok(());
            }

            sleep(POLL_INTERVAL).await;
        }

        // Final check: the tokio runtime may have been busy, causing sleep
        // to overshoot the deadline even though the daemon started in time
        if is_process_alive(expected_pid)
            && let Some(pid) = read_pid(&pid_path)
            && pid == expected_pid
        {
            trace!(
                pid,
                "Daemon PID file detected (after deadline), daemon is ready"
            );

            return Ok(());
        }

        Err(DaemonError::StartTimedOut.into())
    }
}
