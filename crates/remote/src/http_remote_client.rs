use crate::blob::*;
use crate::http_tls::*;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionCacheUpdateCapabilities, ActionResult, CacheCapabilities, ServerCapabilities,
    digest_function,
};
use miette::IntoDiagnostic;
use moon_common::color;
use moon_config::{RemoteCompression, RemoteConfig};
use moon_env_var::GlobalEnvBag;
use moon_hash::Digest;
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
            .gzip(true)
            .zstd(true)
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
            self.config.get_host(),
            self.config.cache.instance_name
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
            color::url(config.get_host()),
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
            .get(format!("{}/status", self.config.get_host()))
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
        unreachable!();
    }

    async fn update_action_result(
        &self,
        action_digest: &Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>> {
        unreachable!();
    }

    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>> {
        unreachable!();
    }

    async fn batch_read_blobs(
        &self,
        action_digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Option<CompressableBlob>>> {
        unreachable!();
    }

    async fn batch_update_blobs(
        &self,
        action_digest: &Digest,
        blobs: Vec<CompressableBlob>,
    ) -> miette::Result<Vec<Option<Digest>>> {
        unreachable!();
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
