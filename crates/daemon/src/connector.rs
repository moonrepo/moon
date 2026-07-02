use crate::daemon_error::DaemonError;
use moon_daemon_client::DaemonClient;
use moon_daemon_utils::{endpoint::*, sys::*};
use std::io::Error;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{Instant, sleep};
use tracing::{debug, instrument, trace, warn};

/// Maximum time to wait for the daemon to become ready after spawning.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Interval between readiness polls.
const POLL_INTERVAL: Duration = Duration::from_millis(150);

/// Maximum time to wait for the daemon to shut down gracefully before killing.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Process-wide single-flight for `start_daemon`. Without this, two
/// concurrent callers (e.g. the background session and the action
/// pipeline's `connect_to_daemon`) can both observe `is_running() == None`
/// before the spawned daemon writes its PID file, both call
/// `cleanup_daemon_files`, and both spawn — racing on the endpoint.
static DAEMON_START_LOCK: Mutex<()> = Mutex::const_new(());

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
    pub async fn connect(&self) -> miette::Result<Option<DaemonClient>> {
        Ok(Some(DaemonClient::connect(&self.daemon_dir).await?))
    }

    pub fn get_log_file(&self) -> PathBuf {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        self.daemon_dir.join(format!("server.{date}.log"))
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
    pub async fn start_daemon(&self, timeout: bool) -> miette::Result<Option<u32>> {
        // Serialize concurrent start attempts within this process. Without
        // this, two callers can both pass the `is_running` check, both
        // `cleanup_daemon_files` (the second removing the first daemon's
        // directory mid-startup), and both spawn — leaving them to race
        // for the endpoint, with one failing to bind and exiting 1.
        let _guard = DAEMON_START_LOCK.lock().await;

        // Re-check under the lock: another caller may have just spawned and
        // written the PID file while we were waiting.
        if let Some(pid) = self.is_running() {
            debug!(pid, "Daemon already running, skipping spawn");

            return Ok(Some(pid));
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

        let mut child = command.spawn().map_err(|error| DaemonError::StartFailed {
            error: Box::new(error),
        })?;

        debug!(
            pid = child.id(),
            "Daemon process spawned, waiting for readiness"
        );

        self.wait_for_ready(&mut child, timeout).await
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
    async fn wait_for_ready(
        &self,
        child: &mut Child,
        timeout: bool,
    ) -> miette::Result<Option<u32>> {
        let expected_pid = child.id();
        let pid_path = get_pid_path(&self.daemon_dir);
        let deadline = Instant::now() + STARTUP_TIMEOUT;

        while Instant::now() < deadline {
            // Check that the direct child is still alive. This avoids an
            // OpenProcess false negative on Windows for a process we spawned.
            if let Some(status) = child.try_wait().map_err(|error| DaemonError::StartFailed {
                error: Box::new(error),
            })? {
                // If another process won a concurrent daemon startup race,
                // reuse it instead of failing this process.
                if DaemonClient::test_connection(&self.daemon_dir).await {
                    let pid = read_pid(&pid_path);

                    trace!(
                        pid = expected_pid,
                        new_pid = pid,
                        "Spawned daemon exited, but another daemon is ready"
                    );

                    return Ok(pid);
                }

                return Err(DaemonError::StartFailed {
                    error: Box::new(Error::other(format!(
                        "Daemon process exited unexpectedly during startup ({status})"
                    ))),
                }
                .into());
            }

            // The spawned process may not be the final daemon process on
            // Windows if the CLI delegates from a global binary to a local one.
            if DaemonClient::test_connection(&self.daemon_dir).await {
                trace!(pid = expected_pid, "Daemon endpoint accepted a connection");

                return Ok(Some(expected_pid));
            }

            sleep(POLL_INTERVAL).await;
        }

        // Final check: the tokio runtime may have been busy, causing sleep
        // to overshoot the deadline even though the daemon started in time
        if DaemonClient::test_connection(&self.daemon_dir).await {
            trace!(pid = expected_pid, "Daemon endpoint accepted a connection");

            return Ok(Some(expected_pid));
        }

        if timeout {
            return Err(DaemonError::StartTimedOut.into());
        }

        warn!("Timed out waiting for the daemon to start!");

        Ok(None)
    }
}
