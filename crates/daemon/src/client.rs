use crate::daemon_error::DaemonError;
use crate::endpoint::*;
use crate::proto::moon_daemon_client::MoonDaemonClient;
use crate::proto::*;
use hyper_util::rt::TokioIo;
use std::path::Path;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use tracing::debug;

pub struct DaemonClient {
    inner: MoonDaemonClient<Channel>,
}

impl DaemonClient {
    /// Connect to a running daemon via its platform-specific endpoint.
    ///
    /// - Unix: connects via Unix domain socket
    /// - Windows: connects via named pipe
    pub async fn connect(workspace_root: &Path, cache_dir: &Path) -> miette::Result<Self> {
        let endpoint = get_endpoint(workspace_root, cache_dir);

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

    pub async fn start(&mut self, workspace_root: &str) -> miette::Result<StartResponse> {
        let response = self
            .inner
            .start(StartRequest {
                workspace_root: workspace_root.to_owned(),
            })
            .await
            .map_err(|error| DaemonError::RpcFailed {
                error: Box::new(error),
            })?;

        Ok(response.into_inner())
    }

    pub async fn stop(&mut self) -> miette::Result<StopResponse> {
        let response =
            self.inner
                .stop(StopRequest {})
                .await
                .map_err(|error| DaemonError::RpcFailed {
                    error: Box::new(error),
                })?;

        Ok(response.into_inner())
    }

    pub async fn status(&mut self) -> miette::Result<StatusResponse> {
        let response =
            self.inner
                .status(StatusRequest {})
                .await
                .map_err(|error| DaemonError::RpcFailed {
                    error: Box::new(error),
                })?;

        Ok(response.into_inner())
    }
}

// https://github.com/hyperium/tonic/blob/master/examples/src/uds/client_with_connector.rs
// https://docs.rs/tokio/latest/tokio/net/windows/named_pipe/index.html

#[cfg(unix)]
async fn connect_channel(endpoint: &str) -> Result<Channel, tonic::transport::Error> {
    let path = endpoint.to_owned();

    // tonic UDS: use a dummy URI and override the connector to open a UnixStream.
    // TokioIo wraps the stream to satisfy hyper's Read/Write traits.
    Endpoint::try_from("http://[::]:50051")
        .unwrap()
        .connect_with_connector(service_fn(move |_: Uri| {
            let path = path.clone();

            async move {
                let stream = tokio::net::UnixStream::connect(path).await?;
                Ok::<_, std::io::Error>(TokioIo::new(stream))
            }
        }))
        .await
}

#[cfg(windows)]
async fn connect_channel(endpoint: &str) -> Result<Channel, tonic::transport::Error> {
    let pipe_name = endpoint.to_owned();

    // tonic Named Pipe: use a dummy URI and override the connector.
    // TokioIo wraps the pipe to satisfy hyper's Read/Write traits.
    Endpoint::try_from("http://[::]:50051")
        .unwrap()
        .connect_with_connector(service_fn(move |_: Uri| {
            let name = pipe_name.clone();
            async move {
                let pipe = tokio::net::windows::named_pipe::ClientOptions::new()
                    .open(&name)
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::ConnectionRefused,
                            format!("Failed to open named pipe: {e}"),
                        )
                    })?;

                Ok::<_, std::io::Error>(TokioIo::new(pipe))
            }
        }))
        .await
}
