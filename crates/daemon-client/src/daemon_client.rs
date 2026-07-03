use crate::daemon_client_error::DaemonClientError;
use hyper_util::rt::TokioIo;
use moon_common::color;
use moon_daemon_proto::{moon_daemon_client::MoonDaemonClient, *};
use moon_daemon_utils::endpoint::*;
use std::future::Future;
use std::io::Error;
use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;
use tonic::{
    Request, Status,
    transport::{Channel, Endpoint, Error as TransportError, Uri},
};
use tower::service_fn;
use tracing::{debug, instrument};

/// Maximum time to establish a connection: transport (socket/pipe) open
/// plus the HTTP/2 handshake. Enforced locally with a timer around the
/// whole connect, since a suspended or stalled daemon can otherwise hang
/// a connect indefinitely.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Deadline for fast control procedures (Status, Start, Stop).
const CONTROL_DEADLINE: Duration = Duration::from_secs(5);

/// Deadline for work procedures (SendWebhook, CleanCache, ArchiveTaskOutputs).
/// Newer daemons queue the work and respond immediately, but older daemons
/// run it inline before responding, so stay generous.
const WORK_DEADLINE: Duration = Duration::from_secs(60);

/// Extra client-side grace on top of the `grpc-timeout` deadline, so the
/// server's own deadline handling gets a chance to respond first with a
/// more descriptive error.
const DEADLINE_GRACE: Duration = Duration::from_secs(1);

/// The outcome of comparing a running daemon's version against the client's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandshakeOutcome {
    /// The daemon matches this client; use it.
    Use,
    /// The daemon is a different moon version or protocol version and should
    /// be stopped and replaced.
    Restart,
}

fn map_rpc_error(error: Status) -> DaemonClientError {
    DaemonClientError::RpcFailed {
        error: Box::new(error),
    }
}

/// Build a request that carries the deadline in the `grpc-timeout` header,
/// which the server enforces by cancelling the handler once it elapses.
fn request_with_deadline<T>(message: T, deadline: Duration) -> Request<T> {
    let mut request = Request::new(message);
    request.set_timeout(deadline);
    request
}

/// Await an RPC with a client-side deadline. The `grpc-timeout` header only
/// instructs the server; this bounds the call locally so a stalled daemon
/// cannot hang the caller.
async fn with_deadline<T, F>(
    procedure: &str,
    deadline: Duration,
    future: F,
) -> Result<T, DaemonClientError>
where
    F: Future<Output = Result<T, Status>>,
{
    debug!("Calling {} procedure", color::property(procedure));

    match timeout(deadline + DEADLINE_GRACE, future).await {
        Ok(result) => result.map_err(map_rpc_error),
        Err(_) => Err(DaemonClientError::RpcTimedOut {
            procedure: procedure.to_owned(),
            timeout_secs: deadline.as_secs(),
        }),
    }
}

#[derive(Clone, Debug)]
pub struct DaemonClient {
    inner: MoonDaemonClient<Channel>,
}

impl DaemonClient {
    /// Connect to a running daemon via its platform-specific endpoint.
    ///
    /// - Unix: connects via Unix domain socket
    /// - Windows: connects via named pipe
    pub async fn connect(daemon_dir: &Path) -> miette::Result<Self> {
        Ok(Self::try_connect(daemon_dir).await?)
    }

    /// Like [`DaemonClient::connect`], but returns the concrete error type
    /// so callers can classify the failure (e.g. to retry when the endpoint
    /// is simply not accepting connections yet).
    pub async fn try_connect(daemon_dir: &Path) -> Result<Self, DaemonClientError> {
        let endpoint = get_endpoint(daemon_dir);

        debug!(endpoint = endpoint, "Connecting to daemon");

        let channel = match timeout(CONNECT_TIMEOUT, connect_channel(&endpoint)).await {
            Ok(Ok(channel)) => {
                debug!(endpoint = endpoint, "Connected to daemon");

                channel
            }
            Ok(Err(error)) => {
                debug!(
                    endpoint = endpoint,
                    error = error.to_string(),
                    "Failed to connect to daemon"
                );

                return Err(DaemonClientError::ConnectFailed {
                    endpoint,
                    error: Box::new(error),
                });
            }
            Err(_) => {
                debug!(endpoint = endpoint, "Timed out connecting to daemon");

                return Err(DaemonClientError::ConnectTimedOut {
                    endpoint,
                    timeout_secs: CONNECT_TIMEOUT.as_secs(),
                });
            }
        };

        Ok(Self {
            inner: MoonDaemonClient::new(channel),
        })
    }

    /// Probe whether a daemon is accepting connections and answering RPCs.
    /// Bounded by the connect timeout and the control deadline, and does
    /// not log, as it's called in tight polling loops.
    pub async fn test_connection(daemon_dir: &Path) -> bool {
        let endpoint = get_endpoint(daemon_dir);

        let Ok(Ok(channel)) = timeout(CONNECT_TIMEOUT, connect_channel(&endpoint)).await else {
            return false;
        };

        with_deadline(
            "Status",
            CONTROL_DEADLINE,
            MoonDaemonClient::new(channel)
                .status(request_with_deadline(StatusRequest {}, CONTROL_DEADLINE)),
        )
        .await
        .is_ok()
    }

    /// Compare a connected daemon's reported version against this client's.
    /// A failure to read status keeps the daemon rather than churning on a
    /// transient error — a real RPC will surface any genuine problem.
    pub async fn handshake(&mut self, client_version: &str) -> HandshakeOutcome {
        debug!("Initiating handshake with daemon");

        match self.status().await {
            Ok(status) => {
                if status.protocol_version == PROTOCOL_VERSION
                    && status.moon_version == client_version
                {
                    debug!(
                        server_version = status.moon_version,
                        client_version,
                        server_protocol = status.protocol_version,
                        client_protocol = PROTOCOL_VERSION,
                        "Daemon version handshake matched"
                    );

                    HandshakeOutcome::Use
                } else {
                    debug!(
                        server_version = status.moon_version,
                        client_version,
                        server_protocol = status.protocol_version,
                        client_protocol = PROTOCOL_VERSION,
                        "Daemon version handshake mismatch"
                    );

                    HandshakeOutcome::Restart
                }
            }
            Err(error) => {
                debug!(
                    error = error.to_string(),
                    "Could not read daemon status for handshake, using it as-is"
                );

                HandshakeOutcome::Use
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn archive_task_outputs(
        &mut self,
        task_target: String,
        hash: String,
    ) -> miette::Result<ArchiveTaskOutputsResponse> {
        let response = with_deadline(
            "ArchiveTaskOutputs",
            WORK_DEADLINE,
            self.inner.archive_task_outputs(request_with_deadline(
                ArchiveTaskOutputsRequest {
                    task_target: task_target.to_owned(),
                    hash: hash.to_owned(),
                },
                WORK_DEADLINE,
            )),
        )
        .await?;

        Ok(response.into_inner())
    }

    #[instrument(skip(self))]
    pub async fn clean_cache(
        &mut self,
        lifetime: String,
        all: bool,
    ) -> miette::Result<CleanCacheResponse> {
        let response = with_deadline(
            "CleanCache",
            WORK_DEADLINE,
            self.inner.clean_cache(request_with_deadline(
                CleanCacheRequest { lifetime, all },
                WORK_DEADLINE,
            )),
        )
        .await?;

        Ok(response.into_inner())
    }

    #[instrument(skip(self))]
    pub async fn send_webhook(
        &mut self,
        url: String,
        body: String,
    ) -> miette::Result<SendWebhookResponse> {
        let response = with_deadline(
            "SendWebhook",
            WORK_DEADLINE,
            self.inner.send_webhook(request_with_deadline(
                SendWebhookRequest { url, body },
                WORK_DEADLINE,
            )),
        )
        .await?;

        Ok(response.into_inner())
    }

    #[instrument(skip(self))]
    pub async fn start(&mut self, workspace_root: String) -> miette::Result<StartResponse> {
        let response = with_deadline(
            "Start",
            CONTROL_DEADLINE,
            self.inner.start(request_with_deadline(
                StartRequest { workspace_root },
                CONTROL_DEADLINE,
            )),
        )
        .await?;

        Ok(response.into_inner())
    }

    #[instrument(skip(self))]
    pub async fn status(&mut self) -> miette::Result<StatusResponse> {
        let response = with_deadline(
            "Status",
            CONTROL_DEADLINE,
            self.inner
                .status(request_with_deadline(StatusRequest {}, CONTROL_DEADLINE)),
        )
        .await?;

        Ok(response.into_inner())
    }

    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> miette::Result<StopResponse> {
        let response = with_deadline(
            "Stop",
            CONTROL_DEADLINE,
            self.inner
                .stop(request_with_deadline(StopRequest {}, CONTROL_DEADLINE)),
        )
        .await?;

        Ok(response.into_inner())
    }
}

// https://github.com/hyperium/tonic/blob/master/examples/src/uds/client_with_connector.rs
// https://docs.rs/tokio/latest/tokio/net/windows/named_pipe/index.html

#[cfg(unix)]
async fn connect_channel(endpoint: &str) -> Result<Channel, TransportError> {
    let path = endpoint.to_owned();

    // tonic UDS: use a dummy URI and override the connector to open a UnixStream.
    // TokioIo wraps the stream to satisfy hyper's Read/Write traits.
    Endpoint::try_from("http://[::]:50051")
        .unwrap()
        .connect_with_connector(service_fn(move |_: Uri| {
            let path = path.clone();

            async move {
                let stream = tokio::net::UnixStream::connect(path).await?;

                Ok::<_, Error>(TokioIo::new(stream))
            }
        }))
        .await
}

#[cfg(windows)]
async fn connect_channel(endpoint: &str) -> Result<Channel, TransportError> {
    use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

    let pipe_name = endpoint.to_owned();

    // tonic Named Pipe: use a dummy URI and override the connector.
    // TokioIo wraps the pipe to satisfy hyper's Read/Write traits.
    Endpoint::try_from("http://[::]:50051")
        .unwrap()
        .connect_with_connector(service_fn(move |_: Uri| {
            let name = pipe_name.clone();

            async move {
                let mut attempts = 0;

                // Every pipe instance may be busy while the server publishes
                // the next one, so wait and retry as the docs instruct:
                // https://docs.rs/tokio/latest/tokio/net/windows/named_pipe/struct.ClientOptions.html#method.open
                loop {
                    match tokio::net::windows::named_pipe::ClientOptions::new().open(&name) {
                        Ok(pipe) => break Ok::<_, Error>(TokioIo::new(pipe)),
                        Err(error)
                            if error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32)
                                && attempts < 20 =>
                        {
                            attempts += 1;

                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        Err(error) => break Err(error),
                    }
                }
            }
        }))
        .await
}
