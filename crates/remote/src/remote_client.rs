use crate::fs_digest::Blob;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, Digest, ServerCapabilities,
};
use moon_config::RemoteConfig;
use std::path::Path;

#[async_trait::async_trait]
pub trait RemoteClient: Send + Sync {
    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        workspace_root: &Path,
    ) -> miette::Result<()>;

    async fn load_capabilities(&self) -> miette::Result<ServerCapabilities>;

    async fn get_action_result(&self, digest: &Digest) -> miette::Result<Option<ActionResult>>;

    async fn update_action_result(
        &self,
        digest: &Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>>;

    async fn batch_read_blobs(
        &self,
        digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Blob>>;

    async fn batch_update_blobs(
        &self,
        digest: &Digest,
        blobs: Vec<Blob>,
    ) -> miette::Result<Vec<Option<Digest>>>;
}
