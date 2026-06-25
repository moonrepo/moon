use crate::compressable_blob::*;
use crate::http_tls::*;
use crate::remote_error::RemoteError;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobContent, BlobSource};
use moon_cache_storage::{CacheCapabilities, Manifest, StorageBackend};
use moon_common::Id;
use moon_config::RemoteCompression;
use moon_config::{CacheConfig, RemoteConfig};
use moon_hash::Digest;
use reqwest::Client;
use reqwest::header::HeaderMap;
use rustc_hash::FxHashSet;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::error;

pub struct HttpRemoteStorage {
    capabilities: CacheCapabilities,

    client: OnceLock<Arc<Client>>,
    config: RemoteConfig,
    debug: bool,
    id: Id,
    workspace_root: PathBuf,

    // Since HTTP doesn't support batching, we will most likely
    // end up up/downloading too many files in parallel, triggering a
    // "too many open files" OS error. To circumvent this, we
    // will use a semaphore and limit the amount of parallels.
    semaphore: Arc<Semaphore>,
}

impl HttpRemoteStorage {
    pub fn new(workspace_root: impl AsRef<Path>, config: &RemoteConfig) -> miette::Result<Self> {
        Ok(Self {
            capabilities: CacheCapabilities::default(),
            id: Id::raw("http-remote-cache"),
            client: OnceLock::new(),
            config: config.to_owned(),
            debug: false,
            workspace_root: workspace_root.as_ref().to_path_buf(),
            semaphore: Arc::new(Semaphore::new(100)),
        })
    }

    fn create_client(&self, headers: HeaderMap) -> miette::Result<Client> {
        let mut client = Client::builder()
            .user_agent("moon")
            .gzip(true)
            .zstd(true)
            .tcp_keepalive(Duration::from_secs(60))
            .default_headers(headers);

        if let Some(mtls) = &self.config.mtls {
            client = create_mtls_config(client, mtls, &self.workspace_root)?
        } else if let Some(tls) = &self.config.tls {
            client = create_tls_config(client, tls, &self.workspace_root)?
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

#[async_trait]
impl StorageBackend for HttpRemoteStorage {
    fn get_capabilities(&self) -> &CacheCapabilities {
        &self.capabilities
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    async fn connect(&mut self) -> miette::Result<Option<CacheCapabilities>> {
        let config = &self.config;

        // debug!(
        //     instance = &config.cache.instance_name,
        //     "Connecting to HTTP host {} {}",
        //     color::url(config.get_host()),
        //     if config.mtls.is_some() {
        //         "(with mTLS)"
        //     } else if config.tls.is_some() {
        //         "(with TLS)"
        //     } else if config.is_bearer_auth() {
        //         "(with auth)"
        //     } else {
        //         "(insecure)"
        //     }
        // );

        // self.debug = GlobalEnvBag::instance().should_debug_remote();
        // self.config = config.to_owned();

        // // Extract headers and abort early if not enabled
        // let Some(headers) = self.extract_headers(config)? else {
        //     return Ok(None);
        // };

        // if self.config.cache.compression != RemoteCompression::None {
        //     self.config.cache.compression = RemoteCompression::None;

        //     debug!("HTTP API does not support compression, disabling");
        // }

        // // Create the client
        // let client = self.create_client(headers)?;

        // // Ignore errors since this endpoint is non-standard
        // if let Ok(response) = client
        //     .get(format!("{}/status", self.config.get_host()))
        //     .send()
        //     .await
        // {
        //     let status = response.status();
        //     let code = status.as_u16();

        //     if !status.is_success() && code != 404 {
        //         return Err(RemoteError::HttpConnectFailed {
        //             code,
        //             reason: status
        //                 .canonical_reason()
        //                 .map(|reason| reason.to_owned())
        //                 .unwrap_or_else(|| String::from("Unknown")),
        //         }
        //         .into());
        //     }
        // }

        // let _ = self.client.set(Arc::new(client));

        Ok(Some(CacheCapabilities::default()))
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
                    Err(map_response_error("retrieve_manifest", response, self.debug).into())
                }
            }
            Err(error) => Err(map_error("retrieve_manifest", error, self.debug).into()),
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
                    Err(map_response_error("store_manifest", response, self.debug).into())
                }
            }
            Err(error) => Err(map_error("store_manifest", error, self.debug).into()),
        }
    }

    async fn find_missing_blobs(
        &self,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>> {
        // No way to query this information, so just assume all are missing?
        Ok(FxHashSet::from_iter(blob_digests))
    }

    async fn retrieve_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Blob>> {
        let mut set = JoinSet::<miette::Result<Option<CompressableBlob>>>::new();
        let debug_enabled = self.debug;

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
                                Ok(Some(CompressableBlob::new(digest, bytes.to_vec())))
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
            if let Some(mut blob) = result.into_diagnostic()?? {
                blob.decompress()?;
                blobs.push(blob.inner);
            }
        }

        Ok(blobs)
    }

    async fn store_blobs(&self, blob_sources: Vec<BlobSource>) -> miette::Result<u16> {
        let mut set = JoinSet::<miette::Result<Option<Digest>>>::new();
        let debug_enabled = self.debug;

        for source in blob_sources {
            let client = self.get_client();
            let url = self.get_endpoint("cas", &source.digest.hash);
            let semaphore = self.semaphore.clone();
            let workspace_root = self.workspace_root.clone();

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

        let mut count = 0;

        while let Some(result) = set.join_next().await {
            if result.into_diagnostic()??.is_some() {
                count += 1;
            }
        }

        Ok(count)
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
