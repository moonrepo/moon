use crate::blob::Blob;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, Digest, ServerCapabilities,
};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use miette::IntoDiagnostic;
use moon_config::RemoteConfig;
use moon_env_var::{EnvSubstitutor, GlobalEnvBag};
use std::path::Path;
use tracing::warn;

#[async_trait::async_trait]
pub trait RemoteClient: Send + Sync {
    fn extract_headers(&self, config: &RemoteConfig) -> miette::Result<Option<HeaderMap>> {
        let mut headers = HeaderMap::default();
        let mut substitutor = EnvSubstitutor::new();

        if let Some(auth) = &config.auth {
            for (key, value) in &auth.headers {
                let value = substitutor.substitute(value);

                headers.insert(
                    HeaderName::from_bytes(key.as_bytes()).into_diagnostic()?,
                    HeaderValue::from_str(&value).into_diagnostic()?,
                );
            }

            if let Some(token_name) = &auth.token {
                let token = GlobalEnvBag::instance().get(token_name).unwrap_or_default();

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

    async fn get_action_result(
        &self,
        action_digest: &Digest,
    ) -> miette::Result<Option<ActionResult>>;

    async fn update_action_result(
        &self,
        action_digest: &Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>>;

    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>>;

    async fn batch_read_blobs(
        &self,
        action_digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Option<Blob>>>;

    async fn stream_read_blob(
        &self,
        action_digest: &Digest,
        blob_digest: Digest,
    ) -> miette::Result<Option<Blob>> {
        let mut result = self
            .batch_read_blobs(action_digest, vec![blob_digest])
            .await?;

        Ok(result.remove(0))
    }

    async fn batch_update_blobs(
        &self,
        action_digest: &Digest,
        blobs: Vec<Blob>,
    ) -> miette::Result<Vec<Option<Digest>>>;

    async fn stream_update_blob(
        &self,
        action_digest: &Digest,
        blob: Blob,
    ) -> miette::Result<Digest> {
        let mut result = self.batch_update_blobs(action_digest, vec![blob]).await?;

        Ok(result.remove(0).unwrap())
    }
}
