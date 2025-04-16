use crate::action_state::ActionState;
use crate::blob::*;
use crate::fs_digest::*;
use crate::grpc_remote_client::GrpcRemoteClient;
use crate::http_remote_client::HttpRemoteClient;
use crate::remote_client::RemoteClient;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, Digest, ServerCapabilities, digest_function,
};
use miette::IntoDiagnostic;
use moon_common::{color, is_ci};
use moon_config::{RemoteApi, RemoteCompression, RemoteConfig};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::task::{JoinHandle, JoinSet};
use tracing::{debug, info, instrument, trace, warn};

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
        if is_ci() && config.is_localhost() {
            debug!(
                host = &config.host,
                "Remote service is configured with a localhost endpoint, but we are in a CI environment; disabling service",
            );

            return Ok(());
        }

        info!(
            docs = "https://github.com/bazelbuild/remote-apis",
            "Remote service, powered by the Bazel Remote Execution API, is currently unstable"
        );
        info!("Please report any issues to GitHub or Discord");

        let mut client: Box<dyn RemoteClient> = match config.api {
            RemoteApi::Grpc => Box::new(GrpcRemoteClient::default()),
            RemoteApi::Http => Box::new(HttpRemoteClient::default()),
        };

        let mut instance = Self {
            cache_enabled: client.connect_to_host(config, workspace_root).await?,
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
                    color::property("unstable_remote.cache.compression"),
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
                    color::property("unstable_remote.cache.compression"),
                );
            }

            if let Some(ac_cap) = &cap.action_cache_update_capabilities {
                if !ac_cap.update_enabled {
                    enabled = false;

                    warn!(
                        host,
                        "Remote service does not support caching of actions, which is required by moon"
                    );
                }
            }
        } else {
            enabled = false;

            warn!(
                host,
                "Remote service does not support caching, disabling in moon"
            );
        }

        self.cache_enabled = enabled;

        // TODO check low_api_version/high_api_version

        Ok(())
    }

    pub fn get_max_batch_size(&self) -> i64 {
        let max = self
            .capabilities
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
            .unwrap_or(4194304);

        // Subtract a chunk from the max size, because when down/uploading blobs,
        // we need to account for the non-blob data in the request/response, like the
        // compression level, digest strings, etc. All of these "add up" and can
        // bump the total body size larger than the actual limit. Is there a better
        // way to handle this? Probably, but for now, just reduce the size by 1%,
        // which is about 42k bytes.
        max - (max as f64 * 0.0125) as i64
    }

    #[instrument(skip(self, state))]
    pub async fn is_action_cached(
        &self,
        state: &ActionState<'_>,
    ) -> miette::Result<Option<ActionResult>> {
        if !self.cache_enabled {
            return Ok(None);
        }

        self.client.get_action_result(&state.digest).await
    }

    #[instrument(skip(self, state))]
    pub async fn save_action(&self, state: &mut ActionState<'_>) -> miette::Result<bool> {
        if !self.cache_enabled {
            return Ok(false);
        }

        let missing = self
            .client
            .find_missing_blobs(vec![state.digest.clone()])
            .await?;

        if missing.contains(&state.digest) {
            // This is where moon differs from the Bazel RE API. In Bazel,
            // we would serialize + hash the `Action` and `Command` types,
            // and upload those. But those types do not match how our hashing
            // works, so instead, we're uploading the bytes of our internal
            // hash manifests. Hopefully this doesn't cause issues!
            self.client
                .batch_update_blobs(
                    &state.digest,
                    vec![Blob::new(state.digest.clone(), state.bytes.clone())],
                )
                .await?;
        }

        Ok(true)
    }

    #[instrument(skip(self, state))]
    pub async fn save_action_result(&self, state: &mut ActionState<'_>) -> miette::Result<bool> {
        if !self.cache_enabled {
            return Ok(false);
        }

        let Some((mut result, blobs)) = state.extract_for_upload() else {
            return Ok(false);
        };

        let client = Arc::clone(&self.client);
        let digest = state.digest.clone();
        let max_size = self.get_max_batch_size();

        self.upload_requests
            .write()
            .await
            .push(tokio::spawn(async move {
                if let Some(metadata) = &mut result.execution_metadata {
                    metadata.output_upload_start_timestamp = create_timestamp(SystemTime::now());
                }

                // Don't save the action result if some of the blobs failed to upload
                match batch_upload_blobs(client.clone(), digest.clone(), blobs, max_size as usize)
                    .await
                {
                    Ok(uploaded) => {
                        if !uploaded {
                            return;
                        }
                    }
                    Err(error) => {
                        warn!(
                            hash = &digest.hash,
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
                        hash = &digest.hash,
                        "Failed to cache action result: {}",
                        color::muted_light(error.to_string()),
                    );
                }
            }));

        // We don't actually know at this point if they all uploaded
        Ok(true)
    }

    #[instrument(skip(self, state))]
    pub async fn restore_action_result(&self, state: &mut ActionState<'_>) -> miette::Result<bool> {
        if !self.cache_enabled {
            return Ok(false);
        }

        let Some(result) = &mut state.action_result else {
            return Ok(false);
        };

        match batch_download_blobs(
            self.client.clone(),
            &state.digest,
            result,
            &self.workspace_root,
            self.get_max_batch_size() as usize,
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
                    hash = &state.digest.hash,
                    "Failed to download blobs and restore action result: {}",
                    color::muted_light(error.to_string()),
                );

                return Ok(false);
            }
        };

        // The stderr/stdout blobs may not have been inlined,
        // so we need to fetch them manually
        let mut stdio_digests = vec![];

        if let Some(stderr_digest) = &result.stderr_digest {
            if result.stderr_raw.is_empty() && stderr_digest.size_bytes > 0 {
                stdio_digests.push(stderr_digest.to_owned());
            }
        }

        if let Some(stdout_digest) = &result.stdout_digest {
            if result.stdout_raw.is_empty() && stdout_digest.size_bytes > 0 {
                stdio_digests.push(stdout_digest.to_owned());
            }
        }

        if !stdio_digests.is_empty() {
            for blob in self
                .client
                .batch_read_blobs(&state.digest, stdio_digests)
                .await?
            {
                let Some(blob) = blob else {
                    continue;
                };

                if result
                    .stderr_digest
                    .as_ref()
                    .is_some_and(|dig| dig == &blob.digest)
                {
                    result.stderr_raw = blob.bytes;
                    continue;
                }

                if result
                    .stdout_digest
                    .as_ref()
                    .is_some_and(|dig| dig == &blob.digest)
                {
                    result.stdout_raw = blob.bytes;
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

async fn batch_upload_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    action_digest: Digest,
    mut blobs: Vec<Blob>,
    max_size: usize,
) -> miette::Result<bool> {
    let missing_digests = client
        .find_missing_blobs(blobs.iter().map(|blob| blob.digest.clone()).collect())
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

        // Streaming
        if group.stream {
            set.spawn(async move {
                client
                    .stream_update_blob(&action_digest, group.items.remove(0))
                    .await
                    .map(|res| vec![Some(res)])
            });

            continue;
        }

        // Not streaming
        if group_total > 1 {
            trace!(
                hash = &action_digest.hash,
                blobs = group.items.len(),
                size = group.size,
                "Batching blobs upload (group {} of {})",
                group_index + 1,
                group_total
            );
        }

        set.spawn(async move { client.batch_update_blobs(&action_digest, group.items).await });
    }

    while let Some(res) = set.join_next().await {
        for maybe_digest in res.into_diagnostic()?? {
            if maybe_digest.is_none() {
                set.abort_all();

                return Ok(false);
            }
        }
    }

    Ok(true)
}

async fn batch_download_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    action_digest: &Digest,
    result: &ActionResult,
    workspace_root: &Path,
    max_size: usize,
) -> miette::Result<bool> {
    let mut file_map = FxHashMap::default();
    let mut blob_digests = vec![];

    // TODO support directories
    for file in &result.output_files {
        if let Some(digest) = &file.digest {
            file_map.insert(&digest.hash, file);
            blob_digests.push(digest.to_owned());
        }
    }

    let digest_groups =
        partition_into_groups(blob_digests, max_size, |dig| dig.size_bytes as usize);

    if digest_groups.is_empty() {
        return Ok(true);
    }

    let group_total = digest_groups.len();
    let mut set = JoinSet::<miette::Result<Vec<Option<Blob>>>>::default();

    for (group_index, mut group) in digest_groups.into_iter() {
        let client = Arc::clone(&client);
        let action_digest = action_digest.to_owned();

        // Streaming
        if group.stream {
            set.spawn(async move {
                client
                    .stream_read_blob(&action_digest, group.items.remove(0))
                    .await
                    .map(|res| vec![res])
            });

            continue;
        }

        // Not streaming
        if group_total > 1 {
            trace!(
                hash = &action_digest.hash,
                blobs = group.items.len(),
                size = group.size,
                max_size,
                "Batching blobs download (group {} of {})",
                group_index + 1,
                group_total
            );
        }

        set.spawn(async move { client.batch_read_blobs(&action_digest, group.items).await });
    }

    while let Some(res) = set.join_next().await {
        for blob in res.into_diagnostic()?? {
            let Some(blob) = blob else {
                set.abort_all();

                return Ok(false);
            };

            if let Some(file) = file_map.get(&blob.digest.hash) {
                write_output_file(workspace_root.join(&file.path), blob.bytes, file)?;
            }
        }
    }

    // Create symlinks after blob files have been written,
    // as the link target may reference one of these outputs
    for link in &result.output_symlinks {
        link_output_file(
            workspace_root.join(&link.target),
            workspace_root.join(&link.path),
            link,
        )?;
    }

    Ok(true)
}

struct Partition<T> {
    pub items: Vec<T>,
    pub size: usize,
    pub stream: bool,
}

fn partition_into_groups<T>(
    items: Vec<T>,
    max_size: usize,
    get_size: impl Fn(&T) -> usize,
) -> BTreeMap<i32, Partition<T>> {
    let mut groups = BTreeMap::<i32, Partition<T>>::default();

    // If the max size is larger than 2mb, we reduce the
    // group overall size by half, so that we divide blobs
    // across multiple groups, allowing them to be parallelized
    // better. Waiting for a 2mb up/download is much slower
    // than waiting for multiple parallel 500kb up/downloads.
    let max_group_size = if max_size >= 4194304 {
        2097144
    } else if max_size > 2097144 {
        max_size / 2
    } else {
        max_size
    };

    for item in items {
        let item_size = get_size(&item);
        let mut index_to_use = -1;
        let mut stream = false;

        // Item is too large, must be streamed
        if item_size >= max_group_size {
            stream = true;
        }
        // Try and find a partition that this item can go into
        else {
            for (index, group) in &groups {
                if !group.stream && (group.size + item_size) <= max_group_size {
                    index_to_use = *index;
                    break;
                }
            }
        }

        // If no partition available, create a new one
        if index_to_use == -1 {
            index_to_use = groups.len() as i32;
        }

        let group = groups.entry(index_to_use).or_insert_with(|| Partition {
            items: vec![],
            size: 0,
            stream: false,
        });
        group.size += item_size;
        group.stream = stream;
        group.items.push(item);
    }

    groups
}
