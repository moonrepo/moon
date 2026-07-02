use crate::daemon_error::DaemonError;
use moon_daemon_client::{DaemonClient, HandshakeOutcome};
use moon_daemon_utils::endpoint::*;
use moon_daemon_utils::lock::DaemonLock;
use moon_daemon_utils::sys::*;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{Instant, sleep};
use tracing::{debug, instrument, trace, warn};

/// Maximum time to wait for the daemon to become ready after spawning.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Interval between readiness and lock polls.
const POLL_INTERVAL: Duration = Duration::from_millis(150);

/// Maximum time to wait for the daemon to shut down (release its ownership
/// lock) before forcefully killing it.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Initial delay between connect retry attempts, doubled per attempt.
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(50);

/// Maximum delay between connect retry attempts.
const CONNECT_RETRY_MAX_DELAY: Duration = Duration::from_millis(500);

/// How long to keep retrying connects while the endpoint is unavailable.
/// Covers a daemon that is spawning in another process or restarting.
const CONNECT_RETRY_TIMEOUT: Duration = Duration::from_millis(2500);

/// Hard ceiling on connect retries. The retry deadline keeps sliding while
/// an in-process `start_daemon` is polling for readiness (which itself is
/// capped at [`STARTUP_TIMEOUT`]), so cap the total wait as well.
const CONNECT_RETRY_CEILING: Duration = Duration::from_secs(15);

/// Process-wide single-flight for `start_daemon`. This serializes spawn
/// attempts between threads in _this_ process; the on-disk spawn lock
/// ([`get_spawn_lock_path`]) serializes across processes. It is also the
/// signal [`DaemonConnector::connect`] watches to stay patient while a start
/// is in progress here.
static DAEMON_START_LOCK: Mutex<()> = Mutex::const_new(());

pub struct DaemonConnector {
    pub daemon_dir: PathBuf,
    pub workspace_root: PathBuf,
    pub cli_version: String,
}

impl DaemonConnector {
    pub fn new(daemon_dir: PathBuf, workspace_root: PathBuf, cli_version: String) -> Self {
        Self {
            daemon_dir,
            workspace_root,
            cli_version,
        }
    }

    /// Connect to the daemon, retrying with backoff while the endpoint is
    /// unavailable (missing, refusing, or timing out). This covers the gap
    /// where the daemon was just spawned — possibly by another process —
    /// but hasn't bound its endpoint yet. Retries are bounded by
    /// [`CONNECT_RETRY_TIMEOUT`], extended while an in-process
    /// [`DaemonConnector::start_daemon`] is polling for readiness, and
    /// capped at [`CONNECT_RETRY_CEILING`] overall.
    #[instrument(skip(self))]
    pub async fn connect(&self) -> miette::Result<Option<DaemonClient>> {
        let started = Instant::now();
        let ceiling = started + CONNECT_RETRY_CEILING;
        let mut deadline = (started + CONNECT_RETRY_TIMEOUT).min(ceiling);
        let mut delay = CONNECT_RETRY_DELAY;
        let mut attempt = 1;

        loop {
            let error = match DaemonClient::try_connect(&self.daemon_dir).await {
                Ok(client) => return Ok(Some(client)),
                Err(error) => error,
            };

            // A start in this process holds the lock while it polls for
            // readiness (up to STARTUP_TIMEOUT), so stay patient while
            // that's still happening.
            if DAEMON_START_LOCK.try_lock().is_err() {
                deadline = (Instant::now() + CONNECT_RETRY_TIMEOUT).min(ceiling);
            }

            if !error.is_endpoint_unavailable() || Instant::now() + delay >= deadline {
                return Err(error.into());
            }

            trace!(
                attempt,
                delay_ms = delay.as_millis() as u64,
                "Daemon endpoint unavailable, retrying connection"
            );

            sleep(delay).await;

            delay = (delay * 2).min(CONNECT_RETRY_MAX_DELAY);
            attempt += 1;
        }
    }

    /// Connect to the daemon with a single attempt and no retries. Use this
    /// on paths that should fail fast when no daemon is accepting
    /// connections, such as stopping an already-dead daemon.
    #[instrument(skip(self))]
    pub async fn connect_once(&self) -> miette::Result<Option<DaemonClient>> {
        Ok(Some(DaemonClient::try_connect(&self.daemon_dir).await?))
    }

    /// Acquire a connected client, starting the daemon if one isn't already
    /// running. This is the single entry point for callers that want to _use_
    /// the daemon: it connects if it can, otherwise spawns — coordinating with
    /// any concurrent start through the spawn lock — and connects once ready.
    /// Every caller sharing this path means there's no ordering dependency
    /// between a background pre-warm and the pipeline that needs the daemon;
    /// whoever gets here first starts it, the rest connect to it.
    ///
    /// A daemon left running across a `moon upgrade` reports a different
    /// version during the handshake; it's stopped and replaced once before
    /// giving up, so callers don't keep talking to a stale binary.
    ///
    /// Returns `Ok(None)`, never an error, when the daemon can't be brought up
    /// within the startup budget, so callers transparently degrade to running
    /// without it.
    #[instrument(skip(self))]
    pub async fn acquire(&self) -> miette::Result<Option<DaemonClient>> {
        let mut restarted = false;

        loop {
            let Some(mut client) = self.acquire_client().await else {
                return Ok(None);
            };

            match client.handshake(&self.cli_version).await {
                HandshakeOutcome::Use => {
                    return Ok(Some(client));
                }
                HandshakeOutcome::Restart => {
                    if restarted {
                        warn!(
                            "Daemon version still mismatched after a restart, continuing without it"
                        );

                        return Ok(None);
                    }

                    debug!("Daemon is a different version, restarting it");

                    // Release our connection before tearing the daemon down.
                    drop(client);

                    if let Err(error) = self.stop_daemon().await {
                        warn!(
                            error = error.to_string(),
                            "Failed to stop the mismatched daemon, continuing without it"
                        );

                        return Ok(None);
                    }

                    restarted = true;
                }
            }
        }
    }

    /// Connect to a running daemon, or spawn one and connect once it's ready.
    /// Degrades to `None` on any failure.
    async fn acquire_client(&self) -> Option<DaemonClient> {
        // Fast path: a daemon is already accepting connections.
        if let Ok(client) = DaemonClient::try_connect(&self.daemon_dir).await {
            return Some(client);
        }

        // Otherwise start one (single-flight across threads and processes)
        // and wait until it's ready.
        match self.start_daemon(false).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                warn!("Timed out bringing up the daemon, continuing without it");

                return None;
            }
            Err(error) => {
                warn!(
                    error = error.to_string(),
                    "Failed to start the daemon, continuing without it"
                );

                return None;
            }
        }

        // Ready now — connect, with the bounded retry covering the small
        // window between readiness and this connection.
        match self.connect().await {
            Ok(client) => client,
            Err(error) => {
                warn!(
                    error = error.to_string(),
                    "Daemon started but could not be reached, continuing without it"
                );

                None
            }
        }
    }

    pub fn get_log_file(&self) -> PathBuf {
        self.daemon_dir.join("server.log")
    }

    pub fn get_state_file(&self) -> PathBuf {
        get_state_path(&self.daemon_dir)
    }

    /// Read the daemon's recorded state (pid, version, ...). Informational
    /// only — a value here does not prove a daemon is alive; use
    /// [`DaemonConnector::is_running`] for that.
    pub fn read_state(&self) -> Option<DaemonInfo> {
        read_state(&self.daemon_dir)
    }

    /// Whether a daemon is actually accepting connections. Liveness is a
    /// successful connection, not a PID probe, which is immune to zombies,
    /// reused PIDs, and cross-user permission errors.
    #[instrument(skip(self))]
    pub async fn is_running(&self) -> bool {
        DaemonClient::test_connection(&self.daemon_dir).await
    }

    #[instrument(skip(self))]
    pub async fn start_daemon(&self, timeout: bool) -> miette::Result<Option<u32>> {
        // Serialize spawn attempts within this process (see DAEMON_START_LOCK).
        let _guard = DAEMON_START_LOCK.lock().await;

        // Already accepting connections? Then a daemon is running; reuse it.
        if self.is_running().await {
            let pid = self.read_state().map(|state| state.pid);

            debug!(?pid, "Daemon already running, skipping spawn");

            return Ok(pid);
        }

        // Cross-process single-flight: another CLI may be spawning right now.
        // If we can't take the spawn lock, one is, so wait for its daemon
        // rather than racing a second spawn.
        let spawn_lock = acquire_lock(get_spawn_lock_path(&self.daemon_dir), STARTUP_TIMEOUT)
            .await
            .map_err(|error| DaemonError::StartFailed {
                error: Box::new(error),
            })?;

        let Some(_spawn_lock) = spawn_lock else {
            debug!("Another process is starting the daemon, waiting for it to become ready");

            return self.wait_for_connection(timeout).await;
        };

        // Re-check under the spawn lock: whoever we were racing may have won.
        if self.is_running().await {
            return Ok(self.read_state().map(|state| state.pid));
        }

        let exe_path = std::env::current_exe().map_err(|error| DaemonError::StartFailed {
            error: Box::new(error),
        })?;

        debug!(exe = ?exe_path, "Spawning daemon process");

        // The spawned server removes any stale socket and writes fresh state
        // under its own ownership lock, so we don't pre-clean here.
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
        let running = self.is_running().await;
        let state = self.read_state();

        if !running && state.is_none() {
            debug!("Daemon not running, nothing to stop");

            return Ok(false);
        }

        // Hold the spawn lock so nobody starts a daemon while we stop this one.
        let _spawn_lock = acquire_lock(get_spawn_lock_path(&self.daemon_dir), SHUTDOWN_TIMEOUT)
            .await
            .map_err(|error| DaemonError::StopFailed {
                error: Box::new(error),
            })?;

        // Ask the daemon to shut down gracefully over RPC.
        if running && let Ok(Some(mut client)) = self.connect_once().await {
            debug!("Requesting graceful shutdown");

            let _ = client.stop().await;
        }

        // Confirm the daemon has released ownership by acquiring its lock —
        // holding it proves the daemon is gone. If it won't release within
        // the timeout, force-kill the recorded pid and confirm again.
        let lock_path = get_lock_path(&self.daemon_dir);

        let owned = match acquire_lock(&lock_path, SHUTDOWN_TIMEOUT)
            .await
            .map_err(|error| DaemonError::StopFailed {
                error: Box::new(error),
            })? {
            Some(lock) => Some(lock),
            None => {
                if let Some(info) = &state {
                    warn!(
                        pid = info.pid,
                        "Graceful shutdown did not complete, killing daemon"
                    );

                    kill_process(info.pid).map_err(|error| DaemonError::StopFailed {
                        error: Box::new(error),
                    })?;
                }

                acquire_lock(&lock_path, SHUTDOWN_TIMEOUT)
                    .await
                    .map_err(|error| DaemonError::StopFailed {
                        error: Box::new(error),
                    })?
            }
        };

        match owned {
            Some(_lock) => {
                // We own the lock, so the daemon is gone. Remove its endpoint
                // files (the lock releases as `_lock` drops right after).
                let _ = cleanup_daemon_files(&self.daemon_dir);

                debug!("Daemon stopped");

                Ok(true)
            }
            None => Err(DaemonError::StopTimedOut.into()),
        }
    }

    /// Poll for the daemon to start accepting connections, without a child
    /// process to watch — used when another process owns the spawn.
    #[instrument(skip(self))]
    async fn wait_for_connection(&self, timeout: bool) -> miette::Result<Option<u32>> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;

        while Instant::now() < deadline {
            if self.is_running().await {
                return Ok(self.read_state().map(|state| state.pid));
            }

            sleep(POLL_INTERVAL).await;
        }

        if timeout {
            return Err(DaemonError::StartTimedOut.into());
        }

        warn!("Timed out waiting for the daemon to start!");

        Ok(None)
    }

    #[instrument(skip(self))]
    async fn wait_for_ready(
        &self,
        child: &mut Child,
        timeout: bool,
    ) -> miette::Result<Option<u32>> {
        let expected_pid = child.id();
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
                    let pid = self.read_state().map(|state| state.pid);

                    trace!(
                        pid = expected_pid,
                        new_pid = ?pid,
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

async fn acquire_lock(
    path: impl AsRef<Path>,
    timeout: Duration,
) -> std::io::Result<Option<DaemonLock>> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(lock) = DaemonLock::try_acquire(path.as_ref())? {
            return Ok(Some(lock));
        }

        if Instant::now() >= deadline {
            return Ok(None);
        }

        sleep(POLL_INTERVAL).await;
    }
}
