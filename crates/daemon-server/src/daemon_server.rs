use crate::daemon_server_error::DaemonServerError;
use crate::daemon_watcher::{start_file_listener, start_file_watcher};
use moon_app_context::AppContext;
use moon_common::color;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_daemon_proto::{
    moon_daemon_server::{MoonDaemon, MoonDaemonServer},
    *,
};
use moon_daemon_utils::endpoint::*;
use moon_daemon_utils::lock::DaemonLock;
use moon_file_watcher::{BoxedFileWatcher, FileEvent};
use moon_notifier::notify_webhook;
use moon_process::ProcessRegistry;
use moon_target::Target;
use moon_task_runner::output_archiver::ArchiveOutcome;
use moon_task_runner::{TaskRunState, output_archiver::OutputArchiver};
use moon_workspace_graph::WorkspaceGraph;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, broadcast};
use tonic::{Request, Response, Status, transport::Server};
use tracing::{debug, error, info, warn};

/// How often the lifecycle monitor checks whether the daemon should retire.
const MONITOR_INTERVAL: Duration = Duration::from_secs(60);

/// How long the daemon may sit without any RPC before it exits on its own, so
/// an abandoned workspace doesn't leave a daemon running indefinitely.
const IDLE_TTL: Duration = Duration::from_secs(4 * 60 * 60);

/// Capacity of the file-event broadcast. Sized to absorb a large burst — a
/// branch switch touching many files — without the listener lagging and
/// dropping events, which could miss a config change. The watcher already
/// excludes `node_modules`/`.git`, so the burst is bounded by tracked files.
const EVENT_CHANNEL_CAPACITY: usize = 16_384;

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
    last_activity: Arc<AtomicU64>,
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
                last_activity: Arc::new(AtomicU64::new(0)),
            }),
            state,
        }
    }

    fn last_activity(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.inner.last_activity)
    }

    fn track_activity(&self, procedure: &str) {
        debug!("Received {} request", color::property(procedure));

        self.inner.last_activity.store(
            self.inner.started_at.elapsed().as_millis() as u64,
            Ordering::Relaxed,
        );
    }
}

#[tonic::async_trait]
impl MoonDaemon for DaemonService {
    async fn archive_task_outputs(
        &self,
        request: Request<ArchiveTaskOutputsRequest>,
    ) -> Result<Response<ArchiveTaskOutputsResponse>, Status> {
        self.track_activity("ArchiveTaskOutputs");

        let req = request.into_inner();

        let target = Target::parse(&req.task_target)
            .map_err(|error| Status::invalid_argument(error.to_string()))?;

        let (app_context, task) = {
            let state = self.state.read().await;
            let task = state
                .workspace_graph
                .get_task(&target)
                .map_err(|error| Status::not_found(error.to_string()))?;

            (Arc::clone(&state.app_context), task)
        };

        tokio::spawn(async move {
            // TODO populate the action digest/bytes!
            let task_state = TaskRunState::new(&app_context, &task);

            let result = match OutputArchiver::new(&app_context, &task) {
                Ok(archiver) => archiver.archive(&req.hash, &task_state).await,
                Err(error) => Err(error),
            };

            match result {
                Ok(outcome) => {
                    debug!(
                        target = target.as_str(),
                        hash = &req.hash,
                        queued = matches!(outcome, ArchiveOutcome::Queued),
                        "Archived task outputs"
                    );
                }
                Err(error) => {
                    warn!(
                        target = target.as_str(),
                        hash = &req.hash,
                        "Failed to archive task outputs: {error}"
                    );
                }
            }
        });

        Ok(Response::new(ArchiveTaskOutputsResponse { archived: true }))
    }

    async fn clean_cache(
        &self,
        request: Request<CleanCacheRequest>,
    ) -> Result<Response<CleanCacheResponse>, Status> {
        self.track_activity("CleanCache");

        let app_context = Arc::clone(&self.state.read().await.app_context);
        let request = request.into_inner();

        let (files_deleted, bytes_saved) = app_context
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
        self.track_activity("HashFiles");

        let app_context = Arc::clone(&self.state.read().await.app_context);

        let files = request
            .into_inner()
            .files
            .into_iter()
            .map(WorkspaceRelativePathBuf::from)
            .collect::<Vec<_>>();

        let hashed_files = app_context
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
        self.track_activity("SendWebhook");

        let SendWebhookRequest { url, body } = request.into_inner();

        // Deliver in the background. The work can outlive the client, which
        // acknowledges immediately and may disconnect — the entire reason it
        // offloads delivery to the daemon instead of sending it inline.
        tokio::spawn(async move {
            match notify_webhook(&url, body, false).await {
                Ok(response) if !response.status().is_success() => {
                    warn!(
                        url = &url,
                        status = response.status().as_u16(),
                        "Webhook endpoint responded with a failure"
                    );
                }
                Err(error) => {
                    warn!(url = &url, "Failed to send webhook: {error}");
                }
                _ => {}
            }
        });

        Ok(Response::new(SendWebhookResponse { success: true }))
    }

    async fn start(
        &self,
        _request: Request<StartRequest>,
    ) -> Result<Response<StartResponse>, Status> {
        self.track_activity("Start");

        Ok(Response::new(StartResponse {
            already_running: true,
            endpoint: self.inner.endpoint.clone(),
            pid: self.inner.pid,
        }))
    }

    async fn stop(&self, _request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        self.track_activity("Stop");

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
        self.track_activity("Status");

        let state = self.state.read().await;
        let uptime_secs = self.inner.started_at.elapsed().as_secs();

        Ok(Response::new(StatusResponse {
            endpoint: self.inner.endpoint.clone(),
            moon_version: state.app_context.cli_version.to_string(),
            pid: self.inner.pid,
            protocol_version: PROTOCOL_VERSION,
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
/// On shutdown the state file and socket are removed and the ownership lock
/// is released.
pub async fn start_daemon_server(
    state: DaemonState,
    watchers: Vec<BoxedFileWatcher<AtomicDaemonState>>,
) -> miette::Result<()> {
    let daemon_dir = state.app_context.daemon_dir.clone();
    let workspace_root = state.app_context.workspace_root.clone();
    let version = state.app_context.cli_version.to_string();
    let endpoint = get_endpoint(&daemon_dir);

    fs::create_dir_all(&daemon_dir)?;

    // Take exclusive ownership of this workspace's daemon. The lock is held
    // for our entire lifetime and released automatically when we exit — even
    // on a crash — so the running daemon is whoever holds it, not a PID we'd
    // have to probe. If another daemon already owns it, defer to it.
    let _ownership =
        match DaemonLock::try_acquire(&get_lock_path(&daemon_dir)).map_err(|error| {
            DaemonServerError::EndpointBindFailed {
                endpoint: endpoint.clone(),
                error: Box::new(error),
            }
        })? {
            Some(lock) => lock,
            None => {
                info!("Another daemon already owns this workspace, exiting");

                return Ok(());
            }
        };

    // We own the workspace now, so any leftover socket is stale (no live
    // owner could still hold the lock) and safe to remove before binding.
    #[cfg(unix)]
    {
        let sock = Path::new(&endpoint);

        if sock.exists() {
            fs::remove_file(sock)?;
        }
    }

    // Move out of the workspace so we don't pin it — on Windows an open working
    // directory blocks the folder from being deleted or renamed. Everything
    // uses the explicit workspace root, not the process cwd.
    if let Err(error) = env::set_current_dir(env::temp_dir()) {
        warn!("Failed to move out of the workspace directory: {error}");
    }

    let pid = std::process::id();

    // Record informational state for `moon daemon status`/`stop`. Ownership
    // is the lock above; this file is never consulted to decide liveness.
    write_state(&daemon_dir, DaemonInfo::new(pid, version, endpoint.clone()))?;

    // Create a new atomic state
    let atomic_state = Arc::new(RwLock::new(state));

    // Single broadcast channel for shutdown
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);
    let mut signal_rx = ProcessRegistry::instance().receive_signal();

    // Spawn the file watcher and listener in the background
    let (event_tx, event_rx) = broadcast::channel::<FileEvent>(EVENT_CHANNEL_CAPACITY);
    let watcher_handle = tokio::spawn(start_file_watcher(
        workspace_root.clone(),
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

    // Retire the daemon on its own when the workspace disappears or it goes
    // unused, so an abandoned workspace doesn't leak a daemon forever.
    let monitor_handle = tokio::spawn(monitor_lifecycle(
        workspace_root,
        service.last_activity(),
        shutdown_tx.clone(),
        shutdown_tx.subscribe(),
    ));

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
        monitor_handle.abort();
    }

    // Wait for the file watcher and listener to finish
    match watcher_handle.await {
        Ok(Err(error)) => error!("File watcher exited with error: {error}"),
        Err(error) => error!("File watcher task panicked: {error}"),
        _ => {}
    };

    if let Err(error) = listener_handle.await
        && !error.is_cancelled()
    {
        error!("File listener task panicked: {error}");
    };

    if let Err(error) = monitor_handle.await
        && !error.is_cancelled()
    {
        error!("Lifecycle monitor task panicked: {error}");
    };

    info!("Daemon server stopped");

    // Remove our endpoint files, then release the lock as `_ownership` drops.
    let _ = cleanup_daemon_files(&daemon_dir);

    serve_result
}

/// Retire the daemon on its own when its workspace is deleted or it goes unused
/// for [`IDLE_TTL`], by triggering the shared shutdown. Runs until shutdown.
async fn monitor_lifecycle(
    workspace_root: PathBuf,
    last_activity: Arc<AtomicU64>,
    shutdown_tx: broadcast::Sender<()>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    // The daemon started ~now, and `last_activity` is measured from the same
    // point, so `reference.elapsed() - last_activity` is the idle duration.
    let reference = Instant::now();
    let mut interval = tokio::time::interval(MONITOR_INTERVAL);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let idle = reference
                    .elapsed()
                    .saturating_sub(Duration::from_millis(last_activity.load(Ordering::Relaxed)));

                if !workspace_root.exists() {
                    info!("Daemon shutting down because its workspace was removed");
                } else if idle >= IDLE_TTL {
                    info!("Daemon shutting down because it has been idle too long");
                } else {
                    continue;
                }

                let _ = shutdown_tx.send(());
                break;
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }
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
