use crate::daemon_error::DaemonError;
use crate::endpoint::*;
use crate::proto::moon_daemon_server::{MoonDaemon, MoonDaemonServer};
use crate::proto::*;
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
    fn new(
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
/// Blocks until the server shuts down (via the `Stop` RPC or signal).
pub async fn start_daemon_server(
    workspace_root: &Path,
    daemon_dir: &Path,
    moon_version: &str,
) -> miette::Result<()> {
    let endpoint = get_endpoint(daemon_dir);

    fs::create_dir_all(daemon_dir)?;

    let pid = std::process::id();
    let pid_path = get_pid_path(daemon_dir);

    write_pid(&pid_path, pid)?;

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

    let service = DaemonService::new(
        workspace_root.to_owned(),
        moon_version.to_owned(),
        endpoint.clone(),
        pid,
        shutdown_tx,
    );

    let shutdown_signal = async move {
        shutdown_rx.recv().await;
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

#[cfg(unix)]
async fn serve_unix(
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
async fn serve_windows(
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
