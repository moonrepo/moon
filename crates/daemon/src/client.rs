use crate::daemon_error::DaemonError;
use crate::endpoint::*;
use crate::proto::moon_daemon_client::MoonDaemonClient;
use crate::proto::*;
use hyper_util::rt::TokioIo;
use moon_common::color;
use std::io::Error;
use std::path::Path;
use std::time::Duration;
use tonic::Status;
use tonic::transport::{Channel, Endpoint, Error as TransportError, Uri};
use tower::service_fn;
use tracing::{debug, instrument};

fn map_rpc_error(error: Status) -> DaemonError {
    DaemonError::RpcFailed {
        error: Box::new(error),
    }
}

pub struct DaemonClient {
    inner: MoonDaemonClient<Channel>,
}

impl DaemonClient {
    /// Connect to a running daemon via its platform-specific endpoint.
    ///
    /// - Unix: connects via Unix domain socket
    /// - Windows: connects via named pipe
    pub async fn connect(daemon_dir: &Path) -> miette::Result<Self> {
        let endpoint = get_endpoint(daemon_dir);

        debug!(endpoint = endpoint, "Connecting to daemon");

        let channel =
            connect_channel(&endpoint)
                .await
                .map_err(|error| DaemonError::ConnectFailed {
                    endpoint,
                    error: Box::new(error),
                })?;

        Ok(Self {
            inner: MoonDaemonClient::new(channel),
        })
    }

    #[instrument(skip(self))]
    pub async fn start(&mut self, workspace_root: &str) -> miette::Result<StartResponse> {
        debug!("Calling {} method", color::property("Start"));

        let response = self
            .inner
            .start(StartRequest {
                workspace_root: workspace_root.to_owned(),
            })
            .await
            .map_err(map_rpc_error)?;

        Ok(response.into_inner())
    }

    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> miette::Result<StopResponse> {
        debug!("Calling {} method", color::property("Stop"));

        let response = self
            .inner
            .stop(StopRequest {})
            .await
            .map_err(map_rpc_error)?;

        Ok(response.into_inner())
    }

    #[instrument(skip(self))]
    pub async fn status(&mut self) -> miette::Result<StatusResponse> {
        debug!("Calling {} method", color::property("Status"));

        let response = self
            .inner
            .status(StatusRequest {})
            .await
            .map_err(map_rpc_error)?;

        Ok(response.into_inner())
    }
}

// https://github.com/hyperium/tonic/blob/master/examples/src/uds/client_with_connector.rs
// https://docs.rs/tokio/latest/tokio/net/windows/named_pipe/index.html
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

#[cfg(unix)]
async fn connect_channel(endpoint: &str) -> Result<Channel, TransportError> {
    let path = endpoint.to_owned();

    // tonic UDS: use a dummy URI and override the connector to open a UnixStream.
    // TokioIo wraps the stream to satisfy hyper's Read/Write traits.
    Endpoint::try_from("http://[::]:50051")
        .unwrap()
        .timeout(CONNECT_TIMEOUT)
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
    let pipe_name = endpoint.to_owned();

    // tonic Named Pipe: use a dummy URI and override the connector.
    // TokioIo wraps the pipe to satisfy hyper's Read/Write traits.
    Endpoint::try_from("http://[::]:50051")
        .unwrap()
        .timeout(CONNECT_TIMEOUT)
        .connect_with_connector(service_fn(move |_: Uri| {
            let name = pipe_name.clone();

            async move {
                let pipe = tokio::net::windows::named_pipe::ClientOptions::new()
                    .open(&name)
                    .map_err(|error| {
                        Error::new(
                            std::io::ErrorKind::ConnectionRefused,
                            format!("Failed to open named pipe: {error}"),
                        )
                    })?;

                Ok::<_, Error>(TokioIo::new(pipe))
            }
        }))
        .await
}
