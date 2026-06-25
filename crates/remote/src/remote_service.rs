use crate::blob::*;
use crate::digest_compat::RemoteDigestExt;
use crate::grpc_remote_client::GrpcRemoteClient;
use crate::helpers::*;
use crate::http_remote_client::HttpRemoteClient;
use crate::remote_client::RemoteClient;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    Action, ActionResult, ServerCapabilities, digest_function,
};
use miette::IntoDiagnostic;
use moon_blob::Blob;
use moon_common::{color, is_ci, is_remote};
use moon_config::{RemoteApi, RemoteCompression, RemoteConfig};
use moon_hash::Digest;
use moon_process::ProcessRegistry;
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::task::{JoinHandle, JoinSet};
use tracing::{instrument, trace, warn};

static INSTANCE: OnceLock<Arc<RemoteService>> = OnceLock::new();

pub struct RemoteService {
    pub config: RemoteConfig,
    pub workspace_root: PathBuf,

    cache_enabled: bool,
    capabilities: ServerCapabilities,
    client: Arc<Box<dyn RemoteClient>>,
    upload_requests: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

impl RemoteService {
    pub fn session() -> Option<Arc<RemoteService>> {
        INSTANCE.get().cloned()
    }

    pub fn is_enabled() -> bool {
        INSTANCE.get().is_some_and(|remote| remote.cache_enabled)
    }

    #[instrument]
    pub async fn connect(config: &RemoteConfig, workspace_root: &Path) -> miette::Result<()> {
        if is_remote() && config.is_localhost() {
            warn!(
                host = &config.host,
                "Remote service is configured with a localhost endpoint, but we are in a CI environment; disabling service",
            );

            return Ok(());
        }

        let mut client: Box<dyn RemoteClient> = match config.api {
            RemoteApi::Grpc => Box::new(GrpcRemoteClient::default()),
            RemoteApi::Http => Box::new(HttpRemoteClient::default()),
        };

        let cache_enabled = match client.connect_to_host(config, workspace_root).await {
            Ok(inner) => inner,
            Err(error) => {
                warn!("{error} Disabling remote service!");
                false
            }
        };

        let mut instance = Self {
            cache_enabled,
            capabilities: ServerCapabilities::default(),
            client: Arc::new(client),
            config: config.to_owned(),
            upload_requests: Arc::new(RwLock::new(vec![])),
            workspace_root: workspace_root.to_owned(),
        };

        instance.validate_capabilities().await?;

        let _ = INSTANCE.set(Arc::new(instance));

        Ok(())
    }

    pub async fn validate_capabilities(&mut self) -> miette::Result<()> {
        let host = &self.config.host;
        let mut enabled = self.cache_enabled;

        if !enabled {
            return Ok(());
        }

        self.capabilities = self.client.load_capabilities().await?;

        if let Some(cap) = &self.capabilities.cache_capabilities {
            let sha256_fn = digest_function::Value::Sha256 as i32;

            if !cap.digest_functions.contains(&sha256_fn) {
                enabled = false;

                warn!(
                    host,
                    "Remote service does not support SHA256 digests, which is required by moon"
                );
            }

            let compression = self.config.cache.compression;
            let compressor = get_compressor(compression);

            if compression != RemoteCompression::None
                && !cap.supported_compressors.contains(&compressor)
            {
                enabled = false;

                warn!(
                    host,
                    "Remote service does not support {} compression for streaming, but it has been configured and enabled through the {} setting",
                    compression,
                    color::property("remote.cache.compression"),
                );
            }

            if compression != RemoteCompression::None
                && !cap.supported_batch_update_compressors.contains(&compressor)
            {
                enabled = false;

                warn!(
                    host,
                    "Remote service does not support {} compression for batching, but it has been configured and enabled through the {} setting",
                    compression,
                    color::property("remote.cache.compression"),
                );
            }

            if let Some(ac_cap) = &cap.action_cache_update_capabilities
                && !ac_cap.update_enabled
            {
                enabled = false;

                warn!(
                    host,
                    "Remote service does not support caching of actions, which is required by moon"
                );
            }
        } else {
            enabled = false;

            warn!(
                host,
                "Remote service does not support caching, disabling in moon"
            );
        }

        self.cache_enabled = enabled;

        Ok(())
    }

    pub fn can_download(&self) -> bool {
        self.cache_enabled
    }

    pub fn can_upload(&self) -> bool {
        self.cache_enabled && (is_ci() || !self.config.cache.local_read_only)
    }

    pub fn get_max_batch_size(&self) -> i64 {
        self.capabilities
            .cache_capabilities
            .as_ref()
            .and_then(|cap| {
                if cap.max_batch_total_size_bytes == 0 {
                    None
                } else {
                    Some(cap.max_batch_total_size_bytes)
                }
            })
            // grpc limit: 4mb
            .unwrap_or(4194304)
    }

    #[instrument(skip(self))]
    pub async fn is_action_cached(
        &self,
        action_digest: &Digest,
    ) -> miette::Result<Option<ActionResult>> {
        unreachable!()
    }

    #[instrument(skip(self, _action, blob))]
    pub async fn save_action(&self, _action: Action, blob: Blob) -> miette::Result<bool> {
        unreachable!()
    }

    #[instrument(skip(self, result, blobs))]
    pub async fn save_action_result(
        &self,
        action_digest: &Digest,
        mut result: ActionResult,
        blobs: Vec<Blob>,
    ) -> miette::Result<bool> {
        unreachable!()
    }

    #[instrument(skip(self, result))]
    pub async fn restore_action_result(
        &self,
        action_digest: &Digest,
        result: &mut ActionResult,
    ) -> miette::Result<bool> {
        unreachable!()
    }

    #[instrument(skip(self))]
    pub async fn wait_for_requests(&self) {
        unreachable!()
    }
}

async fn batch_find_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    action_digest: &Digest,
    blob_digests: Vec<Digest>,
    max_size: usize,
) -> miette::Result<Vec<Digest>> {
    unreachable!()
}

async fn batch_upload_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    action_digest: Digest,
    mut blobs: Vec<CompressableBlob>,
    max_size: usize,
) -> miette::Result<bool> {
    unreachable!()
}

async fn batch_download_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    action_digest: &Digest,
    result: &mut ActionResult,
    max_size: usize,
    verify_integrity: bool,
) -> miette::Result<bool> {
    let mut blob_map = FxHashMap::default();
    let mut blob_digests = vec![];
    let mut seen = FxHashSet::default();

    // Dedupe by digest hash: two output files with identical content
    // reference the same blob, but we only need to download it once.
    for file in &result.output_files {
        if file.contents.is_empty()
            && let Some(digest) = &file.digest
        {
            let local_digest = digest.to_local_digest()?;

            if seen.insert(local_digest.hash.clone()) {
                blob_digests.push(local_digest);
            }
        }
    }

    let digest_groups = partition_into_groups(blob_digests, max_size, |dig| dig.size as usize);
    let group_total = digest_groups.len();
    let mut set = JoinSet::default();

    for (group_index, mut group) in digest_groups.into_iter() {
        let client = Arc::clone(&client);
        let action_digest = action_digest.to_owned();
        let group_key = format!("{}:{group_total}", group_index + 1);

        trace!(
            hash = action_digest.hash.as_str(),
            blobs = group.items.len(),
            size = group.size,
            max_size,
            "Batching blobs download (group {group_key})",
        );

        if group.stream {
            set.spawn(Box::pin(async move {
                client
                    .stream_read_blob(&action_digest, group.items.remove(0))
                    .await
                    .map(|res| (group_key, vec![res]))
            }));
        } else {
            set.spawn(Box::pin(async move {
                client
                    .batch_read_blobs(&action_digest, group.items)
                    .await
                    .map(|res| (group_key, res))
            }));
        }
    }

    let mut signal_receiver = ProcessRegistry::instance().receive_signal();
    let mut abort = false;

    'outer: while let Some(res) = set.join_next().await {
        if signal_receiver.try_recv().is_ok() {
            abort = true;
            break 'outer;
        }

        let (group_key, blobs) = res.into_diagnostic()??;

        trace!(
            hash = action_digest.hash.as_str(),
            blobs = blobs.len(),
            "Batched blobs download (group {group_key})",
        );

        for blob in blobs {
            let Some(blob) = blob else {
                abort = true;
                break 'outer;
            };

            if blob.bytes.len() != blob.digest.size as usize {
                trace!(
                    hash = action_digest.hash.as_str(),
                    expected_size = blob.digest.size,
                    actual_size = blob.bytes.len(),
                    "Integrity failure, mismatched file sizes, unable to write output file",
                );

                abort = true;
                break 'outer;
            } else if verify_integrity {
                let actual_digest = Digest::from_bytes(&blob.bytes)?;

                if actual_digest != blob.digest {
                    trace!(
                        hash = action_digest.hash.as_str(),
                        expected_hash = blob.digest.hash.as_str(),
                        actual_hash = actual_digest.hash.as_str(),
                        "Integrity failure, mismatched digests, unable to write output file",
                    );

                    abort = true;
                    break 'outer;
                }
            }

            // Use a string so that the remote digest can index it
            blob_map.insert(blob.digest.hash.to_string(), blob.inner.bytes);
        }
    }

    if abort {
        set.shutdown().await;

        return Ok(false);
    }

    for file in &mut result.output_files {
        let Some(digest) = &file.digest else {
            continue;
        };

        // Clone (don't remove): a blob may be referenced by multiple
        // output files when they share identical content.
        if let Some(bytes) = blob_map.get(&digest.hash) {
            file.contents = bytes.clone().to_vec();
        } else {
            warn!(
                hash = action_digest.hash.as_str(),
                blob_hash = digest.hash.as_str(),
                output_file = ?file.path,
                "Missing file metadata for blob hash, unable to write output file",
            );

            return Ok(false);
        }
    }

    Ok(true)
}
