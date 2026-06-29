use crate::daemon_server_error::DaemonServerError;
use crate::daemon_watcher::{start_file_listener, start_file_watcher};
use moon_app_context::AppContext;
use moon_common::color;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_daemon_proto::{
    moon_daemon_server::{MoonDaemon, MoonDaemonServer},
    *,
};
use moon_daemon_utils::{endpoint::*, sys::is_process_alive};
use moon_file_watcher::{BoxedFileWatcher, FileEvent};
use moon_notifier::notify_webhook;
use moon_process::ProcessRegistry;
use moon_target::Target;
use moon_task_runner::output_archiver::ArchiveOutcome;
use moon_task_runner::{TaskRunState, output_archiver::OutputArchiver};
use moon_workspace_graph::WorkspaceGraph;
use starbase_utils::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, broadcast};
use tonic::{Request, Response, Status, transport::Server};
use tracing::{debug, error, info};

pub struct DaemonState {
    pub app_context: Arc<AppContext>,
    pub workspace_graph: Arc<WorkspaceGraph>,
}

pub type AtomicDaemonState = Arc<RwLock<DaemonState>>;

struct DaemonServiceInner {
    endpoint: String,
    pid: u32,
    shutdown_tx: broadcast::Sender<()>,
    started_at: Instant,
}

pub struct DaemonService {
    inner: Arc<DaemonServiceInner>,
    state: AtomicDaemonState,
}

impl DaemonService {
    pub fn new(
        state: AtomicDaemonState,
        endpoint: String,
        pid: u32,
        shutdown_tx: broadcast::Sender<()>,
    ) -> Self {
        Self {
            inner: Arc::new(DaemonServiceInner {
                endpoint,
                pid,
                shutdown_tx,
                started_at: Instant::now(),
            }),
            state,
        }
    }
}

#[tonic::async_trait]
impl MoonDaemon for DaemonService {
    async fn archive_task_outputs(
        &self,
        request: Request<ArchiveTaskOutputsRequest>,
    ) -> Result<Response<ArchiveTaskOutputsResponse>, Status> {
        debug!("Received {} request", color::property("ArchiveTaskOutputs"));

        let state = self.state.read().await;
        let req = request.into_inner();

        let target = Target::parse(&req.task_target)
            .map_err(|error| Status::invalid_argument(error.to_string()))?;

        let task = state
            .workspace_graph
            .get_task(&target)
            .map_err(|error| Status::not_found(error.to_string()))?;

        // TODO populate the action digest/bytes!
        let task_state = TaskRunState::new(&state.app_context, &task);

        let archived = OutputArchiver::new(&state.app_context, &task)
            .map_err(|error| Status::unknown(error.to_string()))?
            .archive(&req.hash, &task_state)
            .await
            .map_err(|error| Status::unknown(error.to_string()))?;

        Ok(Response::new(ArchiveTaskOutputsResponse {
            archived: matches!(archived, ArchiveOutcome::Queued),
        }))
    }

    async fn clean_cache(
        &self,
        request: Request<CleanCacheRequest>,
    ) -> Result<Response<CleanCacheResponse>, Status> {
        debug!("Received {} request", color::property("CleanCache"));

        let request = request.into_inner();
        let state = self.state.read().await;

        let (files_deleted, bytes_saved) = state
            .app_context
            .cache_engine
            .clean_stale_cache(&request.lifetime, request.all)
            .await
            .map_err(|error| Status::unknown(error.to_string()))?;

        Ok(Response::new(CleanCacheResponse {
            files_deleted: files_deleted as u32,
            bytes_saved,
        }))
    }

    async fn hash_files(
        &self,
        request: Request<HashFilesRequest>,
    ) -> Result<Response<HashFilesResponse>, Status> {
        debug!("Received {} request", color::property("HashFiles"));

        let state = self.state.read().await;

        let files = request
            .into_inner()
            .files
            .into_iter()
            .map(WorkspaceRelativePathBuf::from)
            .collect::<Vec<_>>();

        let hashed_files = state
            .app_context
            .hash_files(&files)
            .await
            .map_err(|error| Status::unknown(error.to_string()))?;

        Ok(Response::new(HashFilesResponse {
            files: hashed_files
                .into_iter()
                .map(|(path, hash)| (path.to_string(), hash))
                .collect(),
        }))
    }

    async fn send_webhook(
        &self,
        request: Request<SendWebhookRequest>,
    ) -> Result<Response<SendWebhookResponse>, Status> {
        debug!("Received {} request", color::property("SendWebhook"));

        let request = request.into_inner();

        let response = notify_webhook(&request.url, request.body, false)
            .await
            .map_err(|error| Status::unknown(error.to_string()))?;

        Ok(Response::new(SendWebhookResponse {
            success: response.status().is_success(),
        }))
    }

    async fn start(
        &self,
        _request: Request<StartRequest>,
    ) -> Result<Response<StartResponse>, Status> {
        debug!(
            "Received {} request (daemon already running)",
            color::property("Start")
        );

        Ok(Response::new(StartResponse {
            already_running: true,
            endpoint: self.inner.endpoint.clone(),
            pid: self.inner.pid,
        }))
    }

    async fn stop(&self, _request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        debug!(
            "Received {} request, initiating graceful shutdown",
            color::property("Stop")
        );

        self.inner
            .shutdown_tx
            .send(())
            .map_err(|_| Status::internal("Failed to send shutdown signal"))?;

        Ok(Response::new(StopResponse { stopped: true }))
    }

    async fn status(
        &self,
        _request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        debug!("Received {} request", color::property("Status"));

        let state = self.state.read().await;
        let uptime_secs = self.inner.started_at.elapsed().as_secs();

        Ok(Response::new(StatusResponse {
            endpoint: self.inner.endpoint.clone(),
            moon_version: state.app_context.cli_version.to_string(),
            pid: self.inner.pid,
            running: true,
            uptime_secs,
            workspace_root: state.app_context.workspace_root.to_string_lossy().into(),
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
    state: DaemonState,
    watchers: Vec<BoxedFileWatcher<AtomicDaemonState>>,
) -> miette::Result<()> {
    let daemon_dir = state.app_context.daemon_dir.clone();
    let workspace_root = state.app_context.workspace_root.clone();
    let endpoint = get_endpoint(&daemon_dir);

    fs::create_dir_all(&daemon_dir)?;

    // Remove stale endpoint files left by a previous crash, but only
    // if no daemon process is actually alive
    remove_stale_endpoint(&daemon_dir, &endpoint)?;

    let pid = std::process::id();
    let pid_path = get_pid_path(&daemon_dir);

    write_pid(&pid_path, pid)?;

    // Create a new atomic state
    let atomic_state = Arc::new(RwLock::new(state));

    // Single broadcast channel for shutdown
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);
    let mut signal_rx = ProcessRegistry::instance().receive_signal();

    // Spawn the file watcher and listener in the background
    let (event_tx, event_rx) = broadcast::channel::<FileEvent>(1024);
    let watcher_handle = tokio::spawn(start_file_watcher(
        workspace_root,
        event_tx,
        shutdown_tx.subscribe(),
    ));
    let listener_handle = tokio::spawn(start_file_listener(
        atomic_state.clone(),
        watchers,
        event_rx,
        shutdown_tx.subscribe(),
    ));

    // Create the RPC service
    let service = DaemonService::new(atomic_state, endpoint.clone(), pid, shutdown_tx.clone());

    // Merge the RPC-driven shutdown with OS signals so the daemon
    // cleans up regardless of how it is stopped
    let shutdown_signal = async move {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown requested via RPC");
            }
            _ = signal_rx.recv() => {
                // Broadcast so the watcher also receives it
                let _ = shutdown_tx.send(());

                info!("Shutdown requested via OS signal");
            }
        }
    };

    info!(pid, endpoint, "Daemon server starting");

    let serve_result = serve(&endpoint, service, shutdown_signal).await;

    if let Err(error) = &serve_result {
        error!("Daemon server failed: {error}");

        watcher_handle.abort();
        listener_handle.abort();
    }

    // Wait for the file watcher and listener to finish
    match watcher_handle.await {
        Ok(Err(error)) => error!("File watcher exited with error: {error}"),
        Err(error) => error!("File watcher task panicked: {error}"),
        _ => {}
    };

    if let Err(error) = listener_handle.await {
        error!("File listener task panicked: {error}");
    };

    info!("Daemon server stopped");

    let _ = cleanup_daemon_files(&daemon_dir);

    serve_result
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

pub async fn serve(
    endpoint: &str,
    service: DaemonService,
    shutdown_signal: impl std::future::Future<Output = ()>,
) -> miette::Result<()> {
    #[cfg(unix)]
    {
        serve_unix(endpoint, service, shutdown_signal).await
    }

    #[cfg(windows)]
    {
        serve_windows(endpoint, service, shutdown_signal).await
    }
}

#[cfg(unix)]
pub async fn serve_unix(
    endpoint: &str,
    service: DaemonService,
    shutdown_signal: impl std::future::Future<Output = ()>,
) -> miette::Result<()> {
    use moon_daemon_utils::sys::UnixListenerStream;
    use tokio::net::UnixListener;

    let listener =
        UnixListener::bind(endpoint).map_err(|error| DaemonServerError::EndpointBindFailed {
            endpoint: endpoint.to_owned(),
            error: Box::new(error),
        })?;

    let incoming = UnixListenerStream::new(listener);

    Server::builder()
        .serve_with_incoming_shutdown(MoonDaemonServer::new(service), incoming, shutdown_signal)
        .await
        .map_err(|error| DaemonServerError::ServerFailed {
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
    use moon_daemon_utils::sys::get_named_pipe_server_stream;

    Server::builder()
        .serve_with_incoming_shutdown(
            MoonDaemonServer::new(service),
            get_named_pipe_server_stream(endpoint),
            shutdown_signal,
        )
        .await
        .map_err(|error| DaemonServerError::ServerFailed {
            error: Box::new(error),
        })?;

    Ok(())
}
