use crate::fs_digest::*;
use crate::grpc_remote_client::GrpcRemoteClient;
use crate::remote_client::RemoteClient;
use crate::remote_helpers::get_compressor;
use crate::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    digest_function, ActionResult, Digest, ExecutedActionMetadata, ServerCapabilities,
};
use miette::IntoDiagnostic;
use moon_action::Operation;
use moon_common::{color, is_ci};
use moon_config::RemoteConfig;
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

    action_results: scc::HashMap<String, ActionResult>,
    cache_enabled: bool,
    capabilities: ServerCapabilities,
    client: Arc<Box<dyn RemoteClient>>,
    upload_requests: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

impl RemoteService {
    pub fn session() -> Option<Arc<RemoteService>> {
        INSTANCE.get().cloned()
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

        let mut client =
            if config.host.starts_with("http://") || config.host.starts_with("https://") {
                return Err(RemoteError::NoHttpClient.into());
            } else if config.host.starts_with("grpc://") || config.host.starts_with("grpcs://") {
                Box::new(GrpcRemoteClient::default())
            } else {
                return Err(RemoteError::UnknownHostProtocol.into());
            };

        client.connect_to_host(config, workspace_root).await?;

        let mut instance = Self {
            action_results: scc::HashMap::default(),
            capabilities: client.load_capabilities().await?,
            cache_enabled: false,
            client: Arc::new(client),
            config: config.to_owned(),
            upload_requests: Arc::new(RwLock::new(vec![])),
            workspace_root: workspace_root.to_owned(),
        };

        instance.validate_capabilities()?;

        let _ = INSTANCE.set(Arc::new(instance));

        Ok(())
    }

    pub fn validate_capabilities(&mut self) -> miette::Result<()> {
        let host = &self.config.host;
        let mut enabled = true;

        if let Some(cap) = &self.capabilities.cache_capabilities {
            let sha256_fn = digest_function::Value::Sha256 as i32;

            if !cap.digest_functions.contains(&sha256_fn) {
                enabled = false;

                warn!(
                    host,
                    "Remote service does not support SHA256 digests, which is required by moon"
                );
            }

            let compressor = get_compressor(self.config.cache.compression);

            if !cap.supported_compressors.contains(&compressor) {
                enabled = false;

                warn!(
                    host,
                    "Remote service does not support {} compression, but it has been configured and enabled through the {} setting",
                    compressor,
                    color::property("remote.cache.compression"),
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
            // grpc limit: 4mb - buffer
            .unwrap_or(4194304 - (1024 * 10))
    }

    #[instrument(skip(self))]
    pub async fn is_operation_cached(&self, digest: &Digest) -> miette::Result<bool> {
        if !self.cache_enabled {
            return Ok(false);
        }

        if self.action_results.contains_async(&digest.hash).await {
            return Ok(true);
        }

        if let Some(result) = self.client.get_action_result(digest).await? {
            let _ = self
                .action_results
                .insert_async(digest.hash.clone(), result)
                .await;

            return Ok(true);
        }

        Ok(false)
    }

    #[instrument(skip(self, operation))]
    pub async fn save_operation(
        &self,
        digest: &Digest,
        operation: &Operation,
    ) -> miette::Result<()> {
        if !self.cache_enabled || operation.has_failed() {
            return Ok(());
        }

        let operation_label = operation.label().to_owned();

        debug!(
            hash = &digest.hash,
            "Caching {} operation",
            color::muted_light(&operation_label)
        );

        let result = self.create_action_result_from_operation(operation, None)?;
        let digest = digest.to_owned();
        let client = Arc::clone(&self.client);

        self.upload_requests
            .write()
            .await
            .push(tokio::spawn(async move {
                if let Err(error) = client.update_action_result(&digest, result).await {
                    warn!(
                        hash = &digest.hash,
                        "Failed to cache {} operation: {}",
                        color::muted_light(operation_label),
                        color::muted_light(error.to_string()),
                    );
                }
            }));

        Ok(())
    }

    #[instrument(skip(self, operation, outputs))]
    pub async fn save_operation_with_outputs(
        &self,
        digest: &Digest,
        operation: &Operation,
        mut outputs: OutputDigests,
    ) -> miette::Result<()> {
        if !self.cache_enabled || operation.has_failed() {
            return Ok(());
        }

        let operation_label = operation.label().to_owned();

        debug!(
            hash = &digest.hash,
            "Caching {} operation with outputs",
            color::muted_light(&operation_label)
        );

        let mut result = self.create_action_result_from_operation(operation, Some(&mut outputs))?;
        result.output_files = outputs.files;
        result.output_symlinks = outputs.symlinks;
        result.output_directories = outputs.dirs;

        let digest = digest.to_owned();
        let client = Arc::clone(&self.client);
        let max_size = self.get_max_batch_size();

        self.upload_requests
            .write()
            .await
            .push(tokio::spawn(async move {
                if !outputs.blobs.is_empty() {
                    if let Some(metadata) = &mut result.execution_metadata {
                        metadata.output_upload_start_timestamp =
                            create_timestamp(SystemTime::now());
                    }

                    let upload_result = batch_upload_blobs(
                        client.clone(),
                        digest.clone(),
                        outputs.blobs,
                        max_size as usize,
                    )
                    .await;

                    if upload_result.is_err() || upload_result.is_ok_and(|res| !res) {
                        return;
                    }

                    if let Some(metadata) = &mut result.execution_metadata {
                        metadata.output_upload_completed_timestamp =
                            create_timestamp(SystemTime::now());
                    }
                }

                if let Err(error) = client.update_action_result(&digest, result).await {
                    warn!(
                        hash = &digest.hash,
                        "Failed to cache {} operation: {}",
                        color::muted_light(operation_label),
                        color::muted_light(error.to_string()),
                    );
                }
            }));

        Ok(())
    }

    #[instrument(skip(self, operation))]
    pub async fn restore_operation(
        &self,
        digest: &Digest,
        operation: &mut Operation,
    ) -> miette::Result<()> {
        if !self.cache_enabled {
            return Ok(());
        }

        let Some(result) = self.action_results.get_async(&digest.hash).await else {
            return Ok(());
        };

        let operation_label = operation.label().to_owned();
        let has_outputs = !result.output_files.is_empty()
            || !result.output_symlinks.is_empty()
            || !result.output_directories.is_empty();

        if has_outputs {
            debug!(
                hash = &digest.hash,
                "Restoring {} operation with outputs",
                color::muted_light(&operation_label)
            );
        } else {
            debug!(
                hash = &digest.hash,
                "Restoring {} operation",
                color::muted_light(&operation_label)
            );
        }

        if let Some(output) = operation.get_output_mut() {
            output.exit_code = Some(result.exit_code);

            if !result.stderr_raw.is_empty() {
                output.set_stderr(String::from_utf8_lossy(&result.stderr_raw).into());
            }

            if !result.stdout_raw.is_empty() {
                output.set_stdout(String::from_utf8_lossy(&result.stdout_raw).into());
            }
        }

        batch_download_blobs(
            self.client.clone(),
            digest,
            &result,
            &self.workspace_root,
            self.get_max_batch_size() as usize,
        )
        .await?;

        debug!(
            hash = &digest.hash,
            "Restored {} operation",
            color::muted_light(&operation_label)
        );

        Ok(())
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

    fn create_action_result_from_operation(
        &self,
        operation: &Operation,
        outputs: Option<&mut OutputDigests>,
    ) -> miette::Result<ActionResult> {
        let mut result = ActionResult {
            execution_metadata: Some(ExecutedActionMetadata {
                worker: "moon".into(),
                execution_start_timestamp: create_timestamp_from_naive(operation.started_at),
                execution_completed_timestamp: operation
                    .finished_at
                    .and_then(create_timestamp_from_naive),
                ..Default::default()
            }),
            ..Default::default()
        };

        if let Some(exec) = operation.get_output() {
            result.exit_code = exec.exit_code.unwrap_or_default();

            if let Some(outputs) = outputs {
                if let Some(stderr) = &exec.stderr {
                    let blob = Blob::new(stderr.as_bytes().to_owned());

                    result.stderr_digest = Some(blob.digest.clone());
                    outputs.blobs.push(blob);
                }

                if let Some(stdout) = &exec.stdout {
                    let blob = Blob::new(stdout.as_bytes().to_owned());

                    result.stdout_digest = Some(blob.digest.clone());
                    outputs.blobs.push(blob);
                }
            }
        }

        Ok(result)
    }
}

async fn batch_upload_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    digest: Digest,
    blobs: Vec<Blob>,
    max_size: usize,
) -> miette::Result<bool> {
    let blob_groups = partition_into_groups(blobs, max_size, |blob| blob.bytes.len());

    if blob_groups.is_empty() {
        return Ok(false);
    }

    let group_total = blob_groups.len();
    let mut set = JoinSet::default();

    for (group_index, group) in blob_groups.into_iter() {
        let client = Arc::clone(&client);
        let digest = digest.to_owned();

        if group_total > 1 {
            trace!(
                hash = &digest.hash,
                blobs = group.items.len(),
                size = group.size,
                max_size,
                "Batching blobs upload (group {} of {})",
                group_index + 1,
                group_total
            );
        }

        set.spawn(async move {
            if let Err(error) = client.batch_update_blobs(&digest, group.items).await {
                warn!(
                    hash = &digest.hash,
                    group = group_index + 1,
                    "Failed to upload blobs: {}",
                    color::muted_light(error.to_string()),
                );

                return false;
            }

            true
        });
    }

    let results = set.join_all().await;

    Ok(results.into_iter().all(|passed| passed))
}

async fn batch_download_blobs(
    client: Arc<Box<dyn RemoteClient>>,
    digest: &Digest,
    result: &ActionResult,
    workspace_root: &Path,
    max_size: usize,
) -> miette::Result<()> {
    let mut file_map = FxHashMap::default();
    let mut digests = vec![];

    // TODO support directories
    for file in &result.output_files {
        if let Some(digest) = &file.digest {
            file_map.insert(&digest.hash, file);
            digests.push(digest.to_owned());
        }
    }

    let digest_groups = partition_into_groups(digests, max_size, |dig| dig.size_bytes as usize);

    if digest_groups.is_empty() {
        return Ok(());
    }

    let group_total = digest_groups.len();
    let mut set = JoinSet::<miette::Result<Vec<Blob>>>::default();

    for (group_index, group) in digest_groups.into_iter() {
        let client = Arc::clone(&client);
        let digest = digest.to_owned();

        if group_total > 1 {
            trace!(
                hash = &digest.hash,
                blobs = group.items.len(),
                size = group.size,
                max_size,
                "Batching blobs download (group {} of {})",
                group_index + 1,
                group_total
            );
        }

        set.spawn(async move { client.batch_read_blobs(&digest, group.items).await });
    }

    while let Some(res) = set.join_next().await {
        for blob in res.into_diagnostic()?? {
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

    Ok(())
}

struct Partition<T> {
    pub items: Vec<T>,
    pub size: usize,
}

fn partition_into_groups<T>(
    items: Vec<T>,
    max_size: usize,
    get_size: impl Fn(&T) -> usize,
) -> BTreeMap<i32, Partition<T>> {
    let mut groups = BTreeMap::<i32, Partition<T>>::default();

    for item in items {
        let item_size = get_size(&item);
        let mut index_to_use = -1;

        if item_size >= max_size {
            warn!(
                size = item_size,
                max_size,
                "Encountered a blob larger than the max size; this is currently not supported until we support the ByteStream API; aborting"
            );

            return BTreeMap::default();
        }

        // Try and find a partition that this item can go into
        for (index, group) in &groups {
            if group.size + item_size < max_size {
                index_to_use = *index;
                break;
            }
        }

        // If no partition available, create a new one
        if index_to_use == -1 {
            index_to_use = groups.len() as i32;
        }

        let entry = groups.entry(index_to_use).or_insert_with(|| Partition {
            items: vec![],
            size: 0,
        });
        entry.size += item_size;
        entry.items.push(item);
    }

    groups
}
