use bazel_remote_apis::build::bazel::remote::{
    asset::v1::Qualifier,
    execution::v2::{Digest, ServerCapabilities},
};
use moon_config::RemoteConfig;

#[async_trait::async_trait]
pub trait RemoteClient: Send + Sync {
    async fn connect_to_host(&mut self, host: &str, config: &RemoteConfig) -> miette::Result<()>;

    async fn load_capabilities(&mut self) -> miette::Result<ServerCapabilities>;

    async fn upload_blob(&self, hash: &str, bytes: Vec<u8>) -> miette::Result<Digest>;

    async fn create_asset(
        &self,
        digest: Digest,
        qualifiers: Vec<Qualifier>,
    ) -> miette::Result<Digest>;
}
