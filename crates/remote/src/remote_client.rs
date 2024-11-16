use bazel_remote_apis::build::bazel::remote::execution::v2::ServerCapabilities;
use moon_config::RemoteConfig;

#[async_trait::async_trait]
pub trait RemoteClient: Send + Sync {
    async fn connect_to_host(&mut self, host: &str, config: &RemoteConfig) -> miette::Result<()>;

    // async fn load_capabilities(&mut self) -> miette::Result<ServerCapabilities>;
}
