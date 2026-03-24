use crate::daemon_error::DaemonError;
use crate::endpoint::*;
use crate::proto::moon_daemon_server::{MoonDaemon, MoonDaemonServer};
use crate::proto::*;
use crate::sys::is_process_alive;
use moon_process::ProcessRegistry;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, transport::Server};
use tracing::{debug, info};

struct DaemonServiceInner {
    endpoint: String,
    moon_version: String,
    pid: u32,
    shutdown_tx: mpsc::Sender<()>,
    started_at: Instant,
    workspace_root: PathBuf,
}

pub struct DaemonService {
    inner: Arc<DaemonServiceInner>,
}

impl DaemonService {
    pub fn new(
        workspace_root: PathBuf,
        moon_version: String,
        endpoint: String,
        pid: u32,
        shutdown_tx: mpsc::Sender<()>,
    ) -> Self {
        Self {
            inner: Arc::new(DaemonServiceInner {
                endpoint,
                moon_version,
                pid,
                shutdown_tx,
                started_at: Instant::now(),
                workspace_root,
            }),
        }
    }
}

#[tonic::async_trait]
impl MoonDaemon for DaemonService {
    async fn start(
        &self,
        _request: Request<StartRequest>,
    ) -> Result<Response<StartResponse>, Status> {
        debug!("Received start request (daemon already running)");

        Ok(Response::new(StartResponse {
            already_running: true,
            endpoint: self.inner.endpoint.clone(),
            pid: self.inner.pid,
        }))
    }

    async fn stop(&self, _request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        debug!("Received stop request, initiating graceful shutdown");

        self.inner
            .shutdown_tx
            .send(())
            .await
            .map_err(|_| Status::internal("Failed to send shutdown signal"))?;

        Ok(Response::new(StopResponse { stopped: true }))
    }

    async fn status(
        &self,
        _request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let uptime_secs = self.inner.started_at.elapsed().as_secs();

        Ok(Response::new(StatusResponse {
            endpoint: self.inner.endpoint.clone(),
            moon_version: self.inner.moon_version.clone(),
            pid: self.inner.pid,
            running: true,
            uptime_secs,
            workspace_root: self.inner.workspace_root.to_string_lossy().into_owned(),
        }))
    }
}

/// Start the gRPC daemon server, listening on a platform-specific endpoint.
///
/// - Unix: binds a Unix domain socket
/// - Windows: creates a named pipe server
///
/// The server shuts down cleanly on:
/// - A `Stop` RPC call from a client
/// - `SIGINT` or `SIGTERM` (Unix) / `Ctrl+C` (Windows)
///
/// On shutdown the PID file and socket are removed.
pub async fn start_daemon_server(
    workspace_root: &Path,
    daemon_dir: &Path,
    moon_version: &str,
) -> miette::Result<()> {
    let endpoint = get_endpoint(daemon_dir);

    fs::create_dir_all(daemon_dir)?;

    // Remove stale endpoint files left by a previous crash, but only
    // if no daemon process is actually alive
    remove_stale_endpoint(daemon_dir, &endpoint)?;

    let pid = std::process::id();
    let pid_path = get_pid_path(daemon_dir);

    write_pid(&pid_path, pid)?;

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    let mut signal_rx = ProcessRegistry::instance().receive_signal();

    let service = DaemonService::new(
        workspace_root.to_owned(),
        moon_version.to_owned(),
        endpoint.clone(),
        pid,
        shutdown_tx,
    );

    // Merge the RPC-driven shutdown channel with OS signals so the
    // daemon cleans up regardless of how it is stopped
    let shutdown_signal = async move {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown requested via RPC");
            }
            _ = signal_rx.recv() => {
                info!("Shutdown requested via OS signal");
            }
        }
    };

    info!(pid, endpoint, "Daemon server starting");

    #[cfg(unix)]
    serve_unix(&endpoint, service, shutdown_signal).await?;

    #[cfg(windows)]
    serve_windows(&endpoint, service, shutdown_signal).await?;

    info!("Daemon server stopped");

    cleanup_daemon_files(daemon_dir)?;

    Ok(())
}

/// Remove a stale Unix socket (or check a stale PID file on Windows)
/// left behind by a crashed daemon that didn't clean up after itself.
///
/// Only removes files when no daemon process is actually running.
#[allow(unused_variables)]
fn remove_stale_endpoint(daemon_dir: &Path, endpoint: &str) -> miette::Result<()> {
    let pid_path = get_pid_path(daemon_dir);

    // If there's a PID file for a process that's still alive, the
    // endpoint is not stale — bail out
    if let Some(pid) = read_pid(&pid_path) {
        if is_process_alive(pid) {
            return Ok(());
        }

        debug!(pid, "Found stale PID file for dead process, cleaning up");
    }

    // On Unix the socket file itself blocks `bind()`
    #[cfg(unix)]
    {
        let sock = Path::new(endpoint);

        if sock.exists() {
            fs::remove_file(sock)?;
        }
    }

    // Remove the stale PID file too.
    if pid_path.exists() {
        fs::remove_file(&pid_path)?;
    }

    Ok(())
}

#[cfg(unix)]
pub async fn serve_unix(
    endpoint: &str,
    service: DaemonService,
    shutdown_signal: impl std::future::Future<Output = ()>,
) -> miette::Result<()> {
    use tokio::net::UnixListener;
    use tokio_stream::wrappers::UnixListenerStream;

    let listener =
        UnixListener::bind(endpoint).map_err(|error| DaemonError::EndpointBindFailed {
            endpoint: endpoint.to_owned(),
            error: Box::new(error),
        })?;

    let incoming = UnixListenerStream::new(listener);

    Server::builder()
        .serve_with_incoming_shutdown(MoonDaemonServer::new(service), incoming, shutdown_signal)
        .await
        .map_err(|error| DaemonError::ServerFailed {
            error: Box::new(error),
        })?;

    Ok(())
}

#[cfg(windows)]
pub async fn serve_windows(
    endpoint: &str,
    service: DaemonService,
    shutdown_signal: impl std::future::Future<Output = ()>,
) -> miette::Result<()> {
    Server::builder()
        .serve_with_incoming_shutdown(
            MoonDaemonServer::new(service),
            crate::sys::get_named_pipe_server_stream(endpoint),
            shutdown_signal,
        )
        .await
        .map_err(|error| DaemonError::ServerFailed {
            error: Box::new(error),
        })?;

    Ok(())
}
