use crate::blob::*;
use crate::http_tls::*;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionCacheUpdateCapabilities, ActionResult, CacheCapabilities, Digest, ServerCapabilities,
    digest_function,
};
use miette::IntoDiagnostic;
use moon_common::color;
use moon_config::{RemoteCompression, RemoteConfig};
use moon_env_var::GlobalEnvBag;
use reqwest::Client;
use reqwest::header::HeaderMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace};

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
    fn create_client(&self, workspace_root: &Path, headers: HeaderMap) -> miette::Result<Client> {
        let mut client = Client::builder()
            .user_agent("moon")
            .tcp_keepalive(Duration::from_secs(60))
            .default_headers(headers);

        if let Some(mtls) = &self.config.mtls {
            client = create_mtls_config(client, mtls, workspace_root)?
        } else if let Some(tls) = &self.config.tls {
            client = create_tls_config(client, tls, workspace_root)?
        } else if self.config.is_secure_protocol() {
            client = create_native_tls_config(client)?;
        }

        let client = client
            .build()
            .map_err(|error| map_error("create_client", error, self.debug))?;

        Ok(client)
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
}

#[async_trait::async_trait]
impl RemoteClient for HttpRemoteClient {
    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        workspace_root: &Path,
    ) -> miette::Result<bool> {
        debug!(
            instance = &config.cache.instance_name,
            "Connecting to HTTP host {} {}",
            color::url(&config.host),
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

        self.debug = GlobalEnvBag::instance().should_debug_remote();
        self.config = config.to_owned();

        // Extract headers and abort early if not enabled
        let Some(headers) = self.extract_headers(config)? else {
            return Ok(false);
        };

        if self.config.cache.compression != RemoteCompression::None {
            self.config.cache.compression = RemoteCompression::None;

            debug!("HTTP API does not support compression, disabling");
        }

        // Create the client
        let client = self.create_client(workspace_root, headers)?;

        // Ignore errors since this endpoint is non-standard
        if let Ok(response) = client
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

        let _ = self.client.set(Arc::new(client));

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

    async fn get_action_result(
        &self,
        action_digest: &Digest,
    ) -> miette::Result<Option<ActionResult>> {
        trace!(
            hash = &action_digest.hash,
            "Checking for a cached action result"
        );

        match self
            .get_client()
            .get(self.get_endpoint("ac", &action_digest.hash))
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();

                if status.is_success() {
                    let result: ActionResult =
                        response
                            .json()
                            .await
                            .map_err(|error| RemoteError::HttpCallFailed {
                                error: Box::new(error),
                            })?;

                    trace!(
                        hash = &action_digest.hash,
                        files = result.output_files.len(),
                        links = result.output_symlinks.len(),
                        dirs = result.output_directories.len(),
                        exit_code = result.exit_code,
                        "Cache hit on action result"
                    );

                    Ok(Some(result))
                } else if status.as_u16() == 404 {
                    trace!(hash = &action_digest.hash, "Cache miss on action result");

                    Ok(None)
                } else {
                    Err(map_response_error("get_action_result", response, self.debug).into())
                }
            }
            Err(error) => Err(map_error("get_action_result", error, self.debug).into()),
        }
    }

    async fn update_action_result(
        &self,
        action_digest: &Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>> {
        trace!(
            hash = &action_digest.hash,
            files = result.output_files.len(),
            links = result.output_symlinks.len(),
            dirs = result.output_directories.len(),
            exit_code = result.exit_code,
            "Caching action result"
        );

        match self
            .get_client()
            .put(self.get_endpoint("ac", &action_digest.hash))
            .header("Content-Type", "application/json")
            .json(&result)
            .send()
            .await
        {
            Ok(response) => {
                // Doesn't return a response body
                // https://github.com/buchgr/bazel-remote/blob/master/server/http.go#L429
                if response.status().is_success() {
                    trace!(hash = &action_digest.hash, "Cached action result");

                    Ok(Some(result))
                } else {
                    Err(map_response_error("update_action_result", response, self.debug).into())
                }
            }
            Err(error) => Err(map_error("update_action_result", error, self.debug).into()),
        }
    }

    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>> {
        // No way to query this information,
        // so just assume all are missing?
        Ok(blob_digests)
    }

    async fn batch_read_blobs(
        &self,
        action_digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Option<Blob>>> {
        trace!(
            hash = &action_digest.hash,
            compression = self.config.cache.compression.to_string(),
            "Downloading {} output blobs",
            blob_digests.len()
        );

        let mut requests: Vec<JoinHandle<miette::Result<Option<Blob>>>> = vec![];
        let debug_enabled = self.debug;

        for blob_digest in blob_digests {
            let client = self.get_client();
            let url = self.get_endpoint("cas", &blob_digest.hash);
            let semaphore = self.semaphore.clone();

            requests.push(tokio::spawn(async move {
                let Ok(_permit) = semaphore.acquire().await else {
                    return Ok(None);
                };

                match client.get(url).send().await {
                    Ok(response) => {
                        let status = response.status();

                        if status.is_success() {
                            return if let Ok(bytes) = response.bytes().await {
                                Ok(Some(Blob::new(blob_digest, bytes.to_vec())))
                            } else {
                                Ok(None)
                            };
                        }

                        Err(map_response_error("batch_read_blobs", response, debug_enabled).into())
                    }
                    Err(error) => Err(map_error("batch_read_blobs", error, debug_enabled).into()),
                }
            }));
        }

        let mut blobs = vec![];
        let total_count = requests.len();

        for future in requests {
            blobs.push(future.await.into_diagnostic()??);
        }

        trace!(
            hash = &action_digest.hash,
            "Downloaded {} of {} output blobs",
            blobs.len(),
            total_count
        );

        Ok(blobs)
    }

    async fn batch_update_blobs(
        &self,
        action_digest: &Digest,
        blobs: Vec<Blob>,
    ) -> miette::Result<Vec<Option<Digest>>> {
        let compression = self.config.cache.compression;
        let mut requests: Vec<JoinHandle<miette::Result<Option<Digest>>>> = vec![];

        trace!(
            hash = &action_digest.hash,
            compression = compression.to_string(),
            "Uploading {} output blobs",
            blobs.len()
        );

        let debug_enabled = self.debug;

        for blob in blobs {
            let client = self.get_client();
            let url = self.get_endpoint("cas", &blob.digest.hash);
            let semaphore = self.semaphore.clone();

            requests.push(tokio::spawn(async move {
                let Ok(_permit) = semaphore.acquire().await else {
                    return Ok(None);
                };

                match client.put(url).body(blob.bytes).send().await {
                    Ok(response) => {
                        let status = response.status();

                        if status.is_success() {
                            return Ok(Some(blob.digest));
                        }

                        Err(
                            map_response_error("batch_update_blobs", response, debug_enabled)
                                .into(),
                        )
                    }
                    Err(error) => Err(map_error("batch_update_blobs", error, debug_enabled).into()),
                }
            }));
        }

        let mut digests = vec![];
        let mut uploaded_count = 0;

        for future in requests {
            let upload = future.await.into_diagnostic()??;

            if upload.is_some() {
                uploaded_count += 1;
            }

            digests.push(upload);
        }

        trace!(
            hash = &action_digest.hash,
            "Uploaded {} of {} output blobs",
            uploaded_count,
            digests.len()
        );

        Ok(digests)
    }
}

fn map_error(method: &str, error: reqwest::Error, debug: bool) -> RemoteError {
    if debug {
        error!("{method}: {:#?}", error);
    }

    RemoteError::HttpCallFailed {
        error: Box::new(error),
    }
}

fn map_response_error(method: &str, res: reqwest::Response, debug: bool) -> RemoteError {
    if debug {
        error!("{method}: {:#?}", res);
    }

    RemoteError::HttpRequestFailed {
        status: Box::new(res.status()),
    }
}
