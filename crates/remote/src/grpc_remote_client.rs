use crate::remote_client::RemoteClient;
use bazel_remote_apis::build::bazel::remote::execution::v2::ServerCapabilities;
use miette::IntoDiagnostic;
use moon_config::RemoteConfig;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
// use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};

#[derive(Default)]
pub struct GrpcRemoteClient {
    channel: Option<Channel>,
}

#[async_trait::async_trait]
impl RemoteClient for GrpcRemoteClient {
    async fn connect_to_host(&mut self, host: &str, config: &RemoteConfig) -> miette::Result<()> {
        let mut endpoint = Endpoint::from_shared(host.to_owned()).into_diagnostic()?;

        self.channel = Some(endpoint.connect().await.into_diagnostic()?);

        Ok(())
    }

    // async fn load_capabilities(&mut self) -> miette::Result<ServerCapabilities> {}
}
