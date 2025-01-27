use crate::blob::Blob;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, Digest, ServerCapabilities,
};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use miette::IntoDiagnostic;
use moon_config::RemoteConfig;
use std::path::Path;
use tracing::warn;

#[async_trait::async_trait]
pub trait RemoteClient: Send + Sync {
    fn extract_headers(&self, config: &RemoteConfig) -> miette::Result<Option<HeaderMap>> {
        let mut headers = HeaderMap::default();

        if let Some(auth) = &config.auth {
            for (key, value) in &auth.headers {
                headers.insert(
                    HeaderName::from_bytes(key.as_bytes()).into_diagnostic()?,
                    HeaderValue::from_str(value).into_diagnostic()?,
                );
            }

            if let Some(token_name) = &auth.token {
                let token = std::env::var(token_name).unwrap_or_default();

                if token.is_empty() {
                    warn!(
                        "Auth token {} does not exist, unable to authorize for remote service",
                        moon_common::color::property(token_name)
                    );

                    return Ok(None);
                } else {
                    let mut value =
                        HeaderValue::from_str(&format!("Bearer {token}")).into_diagnostic()?;
                    value.set_sensitive(true);

                    headers.insert(
                        HeaderName::from_bytes("Authorization".as_bytes()).into_diagnostic()?,
                        value,
                    );
                }
            }
        }

        Ok(Some(headers))
    }

    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        workspace_root: &Path,
    ) -> miette::Result<bool>;

    async fn load_capabilities(&self) -> miette::Result<ServerCapabilities>;

    async fn get_action_result(&self, digest: &Digest) -> miette::Result<Option<ActionResult>>;

    async fn update_action_result(
        &self,
        digest: &Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>>;

    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>>;

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

    async fn stream_update_blob(
        &self,
        digest: &Digest,
        blob: Blob,
    ) -> miette::Result<Option<Digest>> {
        let mut result = self.batch_update_blobs(digest, vec![blob]).await?;

        Ok(result.remove(0))
    }
}
