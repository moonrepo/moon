use crate::headers::extract_headers;
use crate::http_tls::*;
use crate::remote_error::RemoteError;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobContent, BlobSource, Bytes};
use moon_cache_storage::CacheContext;
use moon_cache_storage::{CacheCapabilities, Manifest, StorageBackend};
use moon_common::{Id, color, is_remote};
use moon_config::RemoteCompression;
use moon_hash::Digest;
use reqwest::Client;
use reqwest::header::HeaderMap;
use rustc_hash::FxHashSet;
use starbase_utils::fs;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{debug, error, warn};

#[derive(Debug)]
pub struct HttpRemoteStorage {
    context: CacheContext,
    id: Id,

    // States
    capabilities: OnceLock<CacheCapabilities>,
    client: OnceLock<Arc<Client>>,

    // Since HTTP doesn't support batching, we will most likely
    // end up up/downloading too many files in parallel, triggering a
    // "too many open files" OS error. To circumvent this, we
    // will use a semaphore and limit the amount of parallels.
    semaphore: Arc<Semaphore>,
}

impl HttpRemoteStorage {
    pub fn new(context: CacheContext) -> miette::Result<Self> {
        Ok(Self {
            capabilities: OnceLock::new(),
            id: Id::raw("http-remote-cache"),
            client: OnceLock::new(),
            context,
            semaphore: Arc::new(Semaphore::new(100)),
        })
    }

    fn create_client(&self, headers: HeaderMap) -> miette::Result<Client> {
        let config = &self.context.remote_config;
        let mut client = Client::builder()
            .user_agent("moon")
            .gzip(true)
            .zstd(true)
            .tcp_keepalive(Duration::from_secs(60))
            .default_headers(headers);

        if let Some(mtls) = &config.mtls {
            client = create_mtls_config(client, mtls, &self.context.workspace_root)?
        } else if let Some(tls) = &config.tls {
            client = create_tls_config(client, tls, &self.context.workspace_root)?
        } else if config.is_secure_protocol() {
            client = create_native_tls_config(client)?;
        }

        let client = client
            .build()
            .map_err(|error| map_error("create_client", error, self.context.remote_debug))?;

        Ok(client)
    }

    fn get_client(&self) -> Arc<Client> {
        Arc::clone(self.client.get_or_init(|| Arc::new(Client::new())))
    }

    fn get_endpoint(&self, path: &str, hash: &str) -> String {
        format!(
            "{}/{}/{path}/{hash}",
            self.context.remote_config.get_host(),
            self.context.remote_config.cache.instance_name
        )
    }
}

#[async_trait]
impl StorageBackend for HttpRemoteStorage {
    fn get_capabilities(&self) -> &CacheCapabilities {
        self.capabilities.get_or_init(CacheCapabilities::default)
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    fn is_enabled(&self) -> bool {
        self.context.remote_config.is_enabled() && self.client.get().is_some()
    }

    async fn connect(&self) -> miette::Result<()> {
        let config = &self.context.remote_config;

        if is_remote() && config.is_localhost() {
            warn!(
                storage = self.get_id().as_str(),
                host = &config.host,
                "Remote service is configured with a localhost endpoint, but we are in a CI environment; disabling service",
            );

            return Ok(());
        }

        debug!(
            storage = self.get_id().as_str(),
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

        // Extract headers and abort early if not enabled
        let Some(headers) = extract_headers(config)? else {
            return Ok(());
        };

        if config.cache.compression != RemoteCompression::None {
            debug!("HTTP API does not support compression, disabling");
        }

        // Create the client
        let client = self.create_client(headers)?;

        // Ignore errors since this endpoint is non-standard
        if let Ok(response) = client
            .get(format!("{}/status", config.get_host()))
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

        Ok(())
    }

    async fn retrieve_manifest(&self, digest: Digest) -> miette::Result<Option<Manifest>> {
        match self
            .get_client()
            .get(self.get_endpoint("ac", &digest.hash))
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();

                if status.is_success() {
                    let manifest: Manifest =
                        response
                            .json()
                            .await
                            .map_err(|error| RemoteError::HttpCallFailed {
                                error: Box::new(error),
                            })?;

                    Ok(Some(manifest))
                } else if status.as_u16() == 404 {
                    Ok(None)
                } else {
                    Err(map_response_error(
                        "retrieve_manifest",
                        response,
                        self.context.remote_debug,
                    )
                    .into())
                }
            }
            Err(error) => {
                Err(map_error("retrieve_manifest", error, self.context.remote_debug).into())
            }
        }
    }

    async fn store_manifest(&self, digest: Digest, manifest: Manifest) -> miette::Result<()> {
        match self
            .get_client()
            .put(self.get_endpoint("ac", &digest.hash))
            .header("Content-Type", "application/json")
            .json(&manifest)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(
                        map_response_error("store_manifest", response, self.context.remote_debug)
                            .into(),
                    )
                }
            }
            Err(error) => Err(map_error("store_manifest", error, self.context.remote_debug).into()),
        }
    }

    async fn find_missing_blobs(
        &self,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>> {
        // No way to query this information, so just assume all are missing?
        Ok(FxHashSet::from_iter(blob_digests))
    }

    async fn retrieve_blobs(
        &self,
        blob_digests: Vec<Digest>,
        _stream: bool,
    ) -> miette::Result<Vec<Blob>> {
        let mut set = JoinSet::<miette::Result<Option<Bytes>>>::new();
        let debug_enabled = self.context.remote_debug;

        for digest in blob_digests {
            let client = self.get_client();
            let url = self.get_endpoint("cas", &digest.hash);
            let semaphore = self.semaphore.clone();

            set.spawn(async move {
                let Ok(_permit) = semaphore.acquire().await else {
                    return Ok(None);
                };

                match client.get(url).send().await {
                    Ok(response) => {
                        let status = response.status();

                        if status.is_success() {
                            return if let Ok(bytes) = response.bytes().await {
                                Ok(Some(bytes))
                            } else {
                                Ok(None)
                            };
                        }

                        Err(map_response_error("retrieve_blobs", response, debug_enabled).into())
                    }
                    Err(error) => Err(map_error("retrieve_blobs", error, debug_enabled).into()),
                }
            });
        }

        let mut blobs = vec![];

        while let Some(result) = set.join_next().await {
            if let Some(bytes) = result.into_diagnostic()?? {
                blobs.push(Blob::try_from(bytes)?);
            }
        }

        Ok(blobs)
    }

    async fn store_blobs(
        &self,
        blob_sources: Vec<BlobSource>,
        _stream: bool,
    ) -> miette::Result<Vec<Digest>> {
        let mut set = JoinSet::<miette::Result<Option<Digest>>>::new();
        let debug_enabled = self.context.remote_debug;

        for source in blob_sources {
            let client = self.get_client();
            let url = self.get_endpoint("cas", &source.digest.hash);
            let semaphore = self.semaphore.clone();
            let workspace_root = self.context.workspace_root.clone();

            set.spawn(async move {
                let Ok(_permit) = semaphore.acquire().await else {
                    return Ok(None);
                };

                let blob = match source.content {
                    BlobContent::Inline(bytes) => Vec::from(bytes),
                    BlobContent::File(rel_path) => {
                        fs::read_file_bytes(rel_path.to_logical_path(workspace_root))?
                    }
                };

                match client.put(url).body(blob).send().await {
                    Ok(response) => {
                        let status = response.status();

                        if status.is_success() {
                            return Ok(Some(source.digest));
                        }

                        Err(map_response_error("store_blobs", response, debug_enabled).into())
                    }
                    Err(error) => Err(map_error("store_blobs", error, debug_enabled).into()),
                }
            });
        }

        let mut digests = vec![];

        while let Some(result) = set.join_next().await {
            if let Some(digest) = result.into_diagnostic()?? {
                digests.push(digest);
            }
        }

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
