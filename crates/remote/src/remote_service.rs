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
use moon_common::{color, is_ci, is_remote};
use moon_config::{RemoteApi, RemoteCompression, RemoteConfig};
use moon_hash::{Blob, Digest};
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
        if !self.can_download() {
            return Ok(None);
        }

        self.client.get_action_result(action_digest).await
    }

    #[instrument(skip(self, _action, blob))]
    pub async fn save_action(&self, _action: Action, blob: Blob) -> miette::Result<bool> {
        if !self.can_upload() {
            return Ok(false);
        }

        let digest = blob.digest.clone();

        if self
            .client
            .find_missing_blobs(vec![digest.clone()])
            .await?
            .contains(&digest)
        {
            self.client
                .batch_update_blobs(&digest, vec![CompressableBlob::from_blob(blob)])
                .await?;
        }

        Ok(true)
    }

    #[instrument(skip(self, result, blobs))]
    pub async fn save_action_result(
        &self,
        action_digest: &Digest,
        mut result: ActionResult,
        blobs: Vec<Blob>,
    ) -> miette::Result<bool> {
        if !self.can_upload() {
            return Ok(false);
        }

        let client = Arc::clone(&self.client);
        let digest = action_digest.to_owned();
        let max_size = self.get_max_batch_size();

        self.upload_requests
            .write()
            .await
            .push(tokio::spawn(Box::pin(async move {
                if let Some(metadata) = &mut result.execution_metadata {
                    metadata.output_upload_start_timestamp = create_timestamp(SystemTime::now());
                }

                // Don't save the action result if some of the blobs failed to upload
                match batch_upload_blobs(
                    client.clone(),
                    digest.clone(),
                    blobs
                        .into_iter()
                        .map(CompressableBlob::from_blob)
                        .collect::<Vec<_>>(),
                    max_size as usize,
                )
                .await
                {
                    Ok(uploaded) => {
                        if !uploaded {
                            return;
                        }
                    }
                    Err(error) => {
                        warn!(
                            hash = digest.hash.as_str(),
                            "Failed to upload blobs and cache action result: {}",
                            color::muted_light(error.to_string()),
                        );

                        return;
                    }
                };

                if let Some(metadata) = &mut result.execution_metadata {
                    metadata.output_upload_completed_timestamp =
                        create_timestamp(SystemTime::now());
                }

                if let Err(error) = client.update_action_result(&digest, result).await {
                    warn!(
                        hash = digest.hash.as_str(),
                        "Failed to cache action result: {}",
                        color::muted_light(error.to_string()),
                    );
                }
            })));

        // We don't actually know at this point if they all uploaded
        Ok(true)
    }

    #[instrument(skip(self, result))]
    pub async fn restore_action_result(
        &self,
        action_digest: &Digest,
        result: &mut ActionResult,
    ) -> miette::Result<bool> {
        if !self.can_download() {
            return Ok(false);
        }

        match batch_download_blobs(
            self.client.clone(),
            action_digest,
            result,
            self.get_max_batch_size() as usize,
            self.config.cache.verify_integrity,
        )
        .await
        {
            Ok(downloaded) => {
                if !downloaded {
                    return Ok(false);
                }
            }
            Err(error) => {
                warn!(
                    hash = action_digest.hash.as_str(),
                    "Failed to download blobs and restore action result: {}",
                    color::muted_light(error.to_string()),
                );

                return Ok(false);
            }
        };

        // The stderr/stdout blobs may not have been inlined,
        // so we need to fetch them manually
        let mut stdio_digests = vec![];

        if let Some(stderr_digest) = &result.stderr_digest
            && result.stderr_raw.is_empty()
            && stderr_digest.size_bytes > 0
        {
            stdio_digests.push(stderr_digest.to_local_digest()?);
        }

        if let Some(stdout_digest) = &result.stdout_digest
            && result.stdout_raw.is_empty()
            && stdout_digest.size_bytes > 0
        {
            stdio_digests.push(stdout_digest.to_local_digest()?);
        }

        if !stdio_digests.is_empty() {
            for blob in self
                .client
                .batch_read_blobs(action_digest, stdio_digests)
                .await?
            {
                let Some(blob) = blob else {
                    continue;
                };

                if result.stderr_digest.as_ref().is_some_and(|dig| {
                    dig.hash.as_str() == blob.digest.hash.as_str()
                        && dig.size_bytes == blob.digest.size
                }) {
                    result.stderr_raw = blob.inner.bytes;
                    continue;
                }

                if result.stdout_digest.as_ref().is_some_and(|dig| {
                    dig.hash.as_str() == blob.digest.hash.as_str()
                        && dig.size_bytes == blob.digest.size
                }) {
                    result.stdout_raw = blob.inner.bytes;
                }
            }
        }

        Ok(true)
    }

    #[instrument(skip(self))]
    pub async fn wait_for_requests(&self) {
        let mut requests = self.upload_requests.write().await;

        for future in requests.drain(0..) {
            // We can ignore the errors because we handle them in
            // the tasks above by logging to the console
            let _ = future.await;
        }
    }
}

async fn batch_find_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    action_digest: &Digest,
    blob_digests: Vec<Digest>,
    max_size: usize,
) -> miette::Result<Vec<Digest>> {
    let digest_groups = partition_into_groups(blob_digests, max_size, |digest| {
        digest.size.to_string().len() + digest.hash.len()
    });

    if digest_groups.is_empty() {
        return Ok(vec![]);
    }

    let group_total = digest_groups.len();
    let mut set = JoinSet::default();

    for (group_index, group) in digest_groups.into_iter() {
        let client = Arc::clone(&client);
        let group_key = format!("{}:{group_total}", group_index + 1);

        trace!(
            hash = action_digest.hash.as_str(),
            blobs = group.items.len(),
            "Batching find blobs (group {group_key})",
        );

        set.spawn(Box::pin(async move {
            client
                .find_missing_blobs(group.items)
                .await
                .map(|res| (group_key, res))
        }));
    }

    let mut missing_digests = vec![];
    let mut signal_receiver = ProcessRegistry::instance().receive_signal();
    let mut abort = false;

    while let Some(res) = set.join_next().await {
        if signal_receiver.try_recv().is_ok() {
            abort = true;
            break;
        }

        let (group_key, digests) = res.into_diagnostic()??;

        trace!(
            hash = action_digest.hash.as_str(),
            digests = digests.len(),
            "Batched find blobs (group {group_key})",
        );

        missing_digests.extend(digests);
    }

    if abort {
        set.shutdown().await;

        return Ok(vec![]);
    }

    Ok(missing_digests)
}

async fn batch_upload_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    action_digest: Digest,
    mut blobs: Vec<CompressableBlob>,
    max_size: usize,
) -> miette::Result<bool> {
    let missing_digests = batch_find_blobs(
        Arc::clone(&client),
        &action_digest,
        blobs.iter().map(|blob| blob.digest.clone()).collect(),
        max_size,
    )
    .await?;

    // All blobs already exist in CAS
    if missing_digests.is_empty() {
        return Ok(true);
    }

    // Otherwise, reduce down the blobs list
    blobs.retain(|blob| missing_digests.contains(&blob.digest));

    let blob_groups = partition_into_groups(blobs, max_size, |blob| blob.bytes.len());

    if blob_groups.is_empty() {
        return Ok(true);
    }

    let group_total = blob_groups.len();
    let mut set = JoinSet::default();

    for (group_index, mut group) in blob_groups.into_iter() {
        let client = Arc::clone(&client);
        let action_digest = action_digest.to_owned();
        let group_key = format!("{}:{group_total}", group_index + 1);

        trace!(
            hash = action_digest.hash.as_str(),
            blobs = group.items.len(),
            size = group.size,
            "Batching blobs upload (group {group_key})",
        );

        if group.stream {
            set.spawn(Box::pin(async move {
                client
                    .stream_update_blob(&action_digest, group.items.remove(0))
                    .await
                    .map(|res| (group_key, vec![Some(res)]))
            }));
        } else {
            set.spawn(Box::pin(async move {
                client
                    .batch_update_blobs(&action_digest, group.items)
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

        let (group_key, digests) = res.into_diagnostic()??;

        trace!(
            hash = action_digest.hash.as_str(),
            digests = digests.len(),
            "Batched blobs upload (group {group_key})",
        );

        for maybe_digest in digests {
            if maybe_digest.is_none() {
                abort = true;
                break 'outer;
            }
        }
    }

    if abort {
        set.shutdown().await;

        return Ok(false);
    }

    Ok(true)
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
            file.contents = bytes.to_owned();
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
