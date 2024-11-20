use crate::fs_digest::{create_timestamp, create_timestamp_from_naive, Blob, OutputDigests};
use crate::grpc_remote_client::GrpcRemoteClient;
use crate::remote_client::RemoteClient;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    digest_function, ActionResult, Digest, ExecutedActionMetadata, ServerCapabilities,
};
use moon_action::Operation;
use moon_common::color;
use moon_config::RemoteConfig;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, instrument, warn};

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

    #[instrument(skip(config))]
    pub async fn new(
        config: &RemoteConfig,
        workspace_root: &Path,
    ) -> miette::Result<Arc<RemoteService>> {
        let mut client =
            if config.host.starts_with("http://") || config.host.starts_with("https://") {
                todo!("TODO http client");
            } else if config.host.starts_with("grpc://") || config.host.starts_with("grpcs://") {
                Box::new(GrpcRemoteClient::default())
            } else {
                todo!("Handle error")
            };

        client.connect_to_host(&config.host, config).await?;

        let mut instance = Self {
            action_results: scc::HashMap::default(),
            capabilities: client.load_capabilities().await?,
            config: config.to_owned(),
            client: Arc::new(client),
            cache_enabled: false,
            upload_requests: Arc::new(RwLock::new(vec![])),
            workspace_root: workspace_root.to_owned(),
        };

        instance.validate_capabilities()?;

        let service = Arc::new(instance);
        let _ = INSTANCE.set(Arc::clone(&service));

        Ok(service)
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

        let result = self
            .create_action_result_from_operation(operation, None)
            .await?;

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

        let mut result = self
            .create_action_result_from_operation(operation, Some(&mut outputs))
            .await?;
        result.output_files = outputs.files;
        result.output_symlinks = outputs.symlinks;
        result.output_directories = outputs.dirs;

        if !outputs.blobs.is_empty() {
            if let Some(metadata) = &mut result.execution_metadata {
                metadata.output_upload_start_timestamp = create_timestamp(SystemTime::now());
            }

            self.client
                .batch_update_blobs(digest, outputs.blobs)
                .await?;

            if let Some(metadata) = &mut result.execution_metadata {
                metadata.output_upload_completed_timestamp = create_timestamp(SystemTime::now());
            }
        }

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

        if let Some(output) = operation.get_output_mut() {
            output.exit_code = Some(result.exit_code);

            if !result.stderr_raw.is_empty() {
                output.set_stderr(String::from_utf8_lossy(&result.stderr_raw).into());
            }

            if !result.stdout_raw.is_empty() {
                output.set_stdout(String::from_utf8_lossy(&result.stdout_raw).into());
            }
        }

        let mut blob_digests = vec![];
        let mut file_map = FxHashMap::default();

        // TODO support symlinks and directories
        for file in &result.output_files {
            if let Some(digest) = &file.digest {
                blob_digests.push(digest.to_owned());
                file_map.insert(&digest.hash, file);
            }
        }

        for blob in self.client.batch_read_blobs(digest, blob_digests).await? {
            if let Some(file) = file_map.get(&blob.digest.hash) {
                fs::write_file(self.workspace_root.join(&file.path), &blob.bytes)?;
            }
        }

        Ok(())
    }

    pub async fn wait_for_requests(&self) {
        let mut requests = self.upload_requests.write().await;

        for future in requests.drain(0..) {
            // We can ignore the errors because we handle them in
            // the tasks above by logging to the console
            let _ = future.await;
        }
    }

    async fn create_action_result_from_operation(
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
