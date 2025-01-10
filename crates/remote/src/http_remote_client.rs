use crate::compression::*;
use crate::fs_digest::Blob;
use crate::http_endpoints::*;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    digest_function, ActionCacheUpdateCapabilities, ActionResult, CacheCapabilities, Digest,
    ExecutionCapabilities, ServerCapabilities,
};
use moon_config::{RemoteCompression, RemoteConfig};
use reqwest::Client;
use std::sync::Arc;
use std::{path::Path, sync::OnceLock};
use tracing::{trace, warn};

#[derive(Default)]
pub struct HttpRemoteClient {
    client: OnceLock<Arc<Client>>,
    compression: RemoteCompression,
    host: String,
    instance_name: String,
}

impl HttpRemoteClient {
    fn get_client(&self) -> Arc<Client> {
        Arc::clone(self.client.get_or_init(|| Arc::new(Client::new())))
    }
}

#[async_trait::async_trait]
impl RemoteClient for HttpRemoteClient {
    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        _workspace_root: &Path,
    ) -> miette::Result<bool> {
        self.compression = config.cache.compression;
        self.host = config.host.clone();
        self.instance_name = config.cache.instance_name.clone();

        // Ignore errors since this endpoint is non-standard
        if let Ok(response) = self
            .get_client()
            .get(format!("{}/status", self.host))
            .send()
            .await
        {
            let status = response.status();
            let code = status.as_u16();

            if !status.is_success() && code != 404 {
                return Err(RemoteError::HttpConnectFailed {
                    code,
                    reason: status
                        .canonical_reason()
                        .map(|reason| reason.to_owned())
                        .unwrap_or_else(|| String::from("Unknown")),
                }
                .into());
            }
        }

        Ok(true)
    }

    // HTTP API doesn't support capabilities, so we need to fake this
    // based on what `bazel-remote` supports
    async fn load_capabilities(&self) -> miette::Result<ServerCapabilities> {
        let digest_functions = vec![digest_function::Value::Sha256 as i32];
        let compressors = get_acceptable_compressors(self.compression);

        Ok(ServerCapabilities {
            cache_capabilities: Some(CacheCapabilities {
                digest_functions: digest_functions.clone(),
                action_cache_update_capabilities: Some(ActionCacheUpdateCapabilities {
                    update_enabled: true,
                }),
                supported_compressors: compressors.clone(),
                supported_batch_update_compressors: compressors,
                ..Default::default()
            }),
            execution_capabilities: Some(ExecutionCapabilities {
                digest_functions,
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    async fn get_action_result(&self, digest: &Digest) -> miette::Result<Option<ActionResult>> {
        trace!(hash = &digest.hash, "Checking for a cached action result");

        match self
            .get_client()
            .get(format!(
                "{}/{}/ac/{}",
                self.host, self.instance_name, digest.hash
            ))
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let result: ActionResult =
                        response
                            .json()
                            .await
                            .map_err(|error| RemoteError::HttpCallFailed {
                                error: Box::new(error),
                            })?;

                    trace!(
                        hash = &digest.hash,
                        files = result.output_files.len(),
                        links = result.output_symlinks.len(),
                        dirs = result.output_directories.len(),
                        exit_code = result.exit_code,
                        "Cache hit on action result"
                    );

                    Ok(Some(result))
                } else {
                    trace!(hash = &digest.hash, "Cache miss on action result");

                    Ok(None)
                }
            }
            Err(error) => Err(RemoteError::HttpCallFailed {
                error: Box::new(error),
            }
            .into()),
        }
    }

    async fn update_action_result(
        &self,
        digest: &Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>> {
        trace!(
            hash = &digest.hash,
            files = result.output_files.len(),
            links = result.output_symlinks.len(),
            dirs = result.output_directories.len(),
            exit_code = result.exit_code,
            "Caching action result"
        );

        match self
            .get_client()
            .put(format!(
                "{}/{}/ac/{}",
                self.host, self.instance_name, digest.hash
            ))
            .header("Content-Type", "application/json")
            .json(&result)
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();

                // Doesn't return a response body
                // https://github.com/buchgr/bazel-remote/blob/master/server/http.go#L429
                if status.is_success() {
                    trace!(hash = &digest.hash, "Cached action result");

                    Ok(Some(result))
                } else {
                    warn!(
                        hash = &digest.hash,
                        code = status.as_u16(),
                        "Failed to cache action result: {}",
                        status
                    );

                    Ok(None)
                }
            }
            Err(error) => Err(RemoteError::HttpCallFailed {
                error: Box::new(error),
            }
            .into()),
        }
    }

    async fn batch_read_blobs(
        &self,
        digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Blob>> {
        Ok(vec![])
    }

    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>> {
        Ok(vec![])
    }

    async fn batch_update_blobs(
        &self,
        digest: &Digest,
        blobs: Vec<Blob>,
    ) -> miette::Result<Vec<Option<Digest>>> {
        trace!(
            hash = &digest.hash,
            compression = self.compression.to_string(),
            "Uploading {} output blobs",
            blobs.len()
        );

        let mut requests = vec![];

        for blob in blobs {
            let client = self.get_client();
            let action_hash = digest.hash.clone();
            let url = format!(
                "{}/{}/cas/{}",
                self.host, self.instance_name, blob.digest.hash
            );

            requests.push(tokio::spawn(async move {
                match client.put(url).body(blob.bytes).send().await {
                    Ok(response) => {
                        let status = response.status();

                        if status.is_success() {
                            return Some(blob.digest);
                        }

                        warn!(
                            hash = &action_hash,
                            blob_hash = &blob.digest.hash,
                            "Failed to upload blob: {status}",
                        );
                    }
                    Err(error) => {
                        warn!(
                            hash = &action_hash,
                            blob_hash = &blob.digest.hash,
                            "Failed to upload blob: {error}",
                        );
                    }
                }

                None
            }));
        }

        let mut digests = vec![];
        let mut uploaded_count = 0;

        for future in requests {
            match future.await {
                Ok(upload) => {
                    if upload.is_some() {
                        uploaded_count += 1;
                    }

                    digests.push(upload);
                }
                _ => {
                    digests.push(None);
                }
            }
        }

        trace!(
            hash = &digest.hash,
            "Uploaded {} of {} output blobs",
            uploaded_count,
            digests.len()
        );

        Ok(vec![])
    }
}
