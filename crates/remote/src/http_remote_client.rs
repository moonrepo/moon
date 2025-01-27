use crate::compression::*;
use crate::fs_digest::Blob;
use crate::http_tls::*;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    digest_function, ActionCacheUpdateCapabilities, ActionResult, CacheCapabilities, Digest,
    ServerCapabilities,
};
use miette::IntoDiagnostic;
use moon_common::color;
use moon_config::{RemoteCompression, RemoteConfig};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;
use starbase_utils::env::bool_var;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use std::{path::Path, sync::OnceLock};
use tokio::sync::Semaphore;
use tracing::{debug, error, trace, warn};

pub struct HttpRemoteClient {
    client: OnceLock<Arc<Client>>,
    config: RemoteConfig,
    debug: bool,

    // Since HTTP doesn't support batching, we will most likely
    // end up up/downloading too many files in parallel, triggering a
    // "too many open files" OS error. To circumvent this, we
    // will use a semaphore and limit the amount of parallels.
    semaphore: Arc<Semaphore>,
}

impl Default for HttpRemoteClient {
    fn default() -> Self {
        Self {
            client: OnceLock::new(),
            config: RemoteConfig::default(),
            debug: false,
            semaphore: Arc::new(Semaphore::new(100)),
        }
    }
}

impl HttpRemoteClient {
    fn create_client(&self, workspace_root: &Path) -> miette::Result<Option<Client>> {
        let mut client = Client::builder()
            .user_agent("moon")
            .tcp_keepalive(Duration::from_secs(60));

        if let Some(auth) = &self.config.auth {
            let mut headers = HeaderMap::default();

            for (key, value) in &auth.headers {
                headers.insert(
                    HeaderName::from_bytes(key.as_bytes()).into_diagnostic()?,
                    HeaderValue::from_str(value).into_diagnostic()?,
                );
            }

            if let Some(token_name) = &auth.token {
                let token = env::var(token_name).unwrap_or_default();

                if token.is_empty() {
                    warn!(
                        "Auth token {} does not exist, unable to authorize for remote service",
                        color::property(token_name)
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

            client = client.default_headers(headers);
        }

        if let Some(mtls) = &self.config.mtls {
            client = create_mtls_config(client, mtls, workspace_root)?
        } else if let Some(tls) = &self.config.tls {
            client = create_tls_config(client, tls, workspace_root)?
        } else if self.config.is_secure_protocol() {
            client = create_native_tls_config(client)?;
        }

        let client = client
            .build()
            .map_err(|error| self.map_error("create_client", error))?;

        Ok(Some(client))
    }

    fn get_client(&self) -> Arc<Client> {
        Arc::clone(self.client.get_or_init(|| Arc::new(Client::new())))
    }

    fn get_endpoint(&self, path: &str, hash: &str) -> String {
        format!(
            "{}/{}/{path}/{hash}",
            self.config.host, self.config.cache.instance_name
        )
    }

    fn map_error(&self, method: &str, error: reqwest::Error) -> RemoteError {
        if self.debug {
            error!("{method}: {:#?}", error);
        }

        RemoteError::HttpCallFailed {
            error: Box::new(error),
        }
    }
}

#[async_trait::async_trait]
impl RemoteClient for HttpRemoteClient {
    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        workspace_root: &Path,
    ) -> miette::Result<bool> {
        self.debug = bool_var("MOON_DEBUG_REMOTE");

        let host = &config.host;

        debug!(
            instance = &config.cache.instance_name,
            "Connecting to HTTP host {} {}",
            color::url(host),
            if config.mtls.is_some() {
                "(with mTLS)"
            } else if config.tls.is_some() {
                "(with TLS)"
            } else if config.is_bearer_auth() {
                "(with auth)"
            } else {
                "(insecure)"
            }
        );

        self.config = config.to_owned();

        if self.config.cache.compression != RemoteCompression::None {
            self.config.cache.compression = RemoteCompression::None;

            debug!("HTTP API does not support compression, disabling");
        }

        // Create client and abort early if not enabled
        match self.create_client(workspace_root)? {
            Some(client) => {
                let _ = self.client.set(Arc::new(client));
            }
            None => {
                return Ok(false);
            }
        }

        // Ignore errors since this endpoint is non-standard
        if let Ok(response) = self
            .get_client()
            .get(format!("{}/status", self.config.host))
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
        let compressors = get_acceptable_compressors(RemoteCompression::None);

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
            execution_capabilities: None,
            ..Default::default()
        })
    }

    async fn get_action_result(&self, digest: &Digest) -> miette::Result<Option<ActionResult>> {
        trace!(hash = &digest.hash, "Checking for a cached action result");

        match self
            .get_client()
            .get(self.get_endpoint("ac", &digest.hash))
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
            Err(error) => Err(self.map_error("get_action_result", error).into()),
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
            .put(self.get_endpoint("ac", &digest.hash))
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
            Err(error) => Err(self.map_error("update_action_result", error).into()),
        }
    }

    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>> {
        // No way to query this information,
        // so just assume all are missing?
        Ok(blob_digests)
    }

    async fn batch_read_blobs(
        &self,
        digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Blob>> {
        trace!(
            hash = &digest.hash,
            compression = self.config.cache.compression.to_string(),
            "Downloading {} output blobs",
            blob_digests.len()
        );

        let mut requests = vec![];
        let debug_enabled = self.debug;

        for blob_digest in blob_digests {
            let client = self.get_client();
            let action_hash = digest.hash.clone();
            let url = self.get_endpoint("cas", &blob_digest.hash);
            let semaphore = self.semaphore.clone();

            requests.push(tokio::spawn(async move {
                let Ok(_permit) = semaphore.acquire().await else {
                    return None;
                };

                match client.get(url).send().await {
                    Ok(response) => {
                        let status = response.status();

                        if status.is_success() {
                            if let Ok(bytes) = response.bytes().await {
                                return Some(Blob {
                                    digest: blob_digest,
                                    bytes: bytes.to_vec(),
                                });
                            }
                        }

                        warn!(
                            hash = &action_hash,
                            blob_hash = &blob_digest.hash,
                            "Failed to download blob: {status}",
                        );
                    }
                    Err(error) => {
                        warn!(
                            hash = &action_hash,
                            blob_hash = &blob_digest.hash,
                            "Failed to download blob: {error}",
                        );

                        if debug_enabled {
                            trace!("read_blob: {:?}", error);
                        }
                    }
                }

                None
            }));
        }

        let mut blobs = vec![];
        let total_count = requests.len();

        for future in requests {
            if let Ok(Some(blob)) = future.await {
                blobs.push(blob);
            }
        }

        trace!(
            hash = &digest.hash,
            "Downloaded {} of {} output blobs",
            blobs.len(),
            total_count
        );

        Ok(blobs)
    }

    async fn batch_update_blobs(
        &self,
        digest: &Digest,
        blobs: Vec<Blob>,
    ) -> miette::Result<Vec<Option<Digest>>> {
        let compression = self.config.cache.compression;
        let mut requests = vec![];

        trace!(
            hash = &digest.hash,
            compression = compression.to_string(),
            "Uploading {} output blobs",
            blobs.len()
        );

        let debug_enabled = self.debug;

        for blob in blobs {
            let client = self.get_client();
            let action_hash = digest.hash.clone();
            let url = self.get_endpoint("cas", &blob.digest.hash);
            let semaphore = self.semaphore.clone();

            requests.push(tokio::spawn(async move {
                let Ok(_permit) = semaphore.acquire().await else {
                    return None;
                };

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

                        if debug_enabled {
                            trace!("update_blob: {:?}", error);
                        }
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

        Ok(digests)
    }
}
