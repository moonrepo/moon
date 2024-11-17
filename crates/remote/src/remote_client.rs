use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, Digest, ServerCapabilities,
};
use moon_config::RemoteConfig;

#[async_trait::async_trait]
pub trait RemoteClient: Send + Sync {
    async fn connect_to_host(&mut self, host: &str, config: &RemoteConfig) -> miette::Result<()>;

    async fn load_capabilities(&self) -> miette::Result<ServerCapabilities>;

    async fn get_action_result(&self, digest: Digest) -> miette::Result<Option<ActionResult>>;

    async fn update_action_result(
        &self,
        digest: Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>>;

    async fn batch_update_blobs(&self, blobs: Vec<Vec<u8>>) -> miette::Result<Vec<Option<Digest>>>;
}
