use crate::fs_digest::{create_timestamp, create_timestamp_from_naive, Blob, OutputDigests};
use crate::grpc_remote_client::GrpcRemoteClient;
use crate::remote_client::RemoteClient;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    digest_function, ActionResult, Digest, ExecutedActionMetadata, ServerCapabilities,
};
use moon_action::{Operation, OperationMeta};
use moon_common::color;
use moon_config::RemoteConfig;
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use tracing::{debug, instrument, warn};

static INSTANCE: OnceLock<Arc<RemoteService>> = OnceLock::new();

pub struct RemoteService {
    pub config: RemoteConfig,

    action_results: scc::HashMap<String, ActionResult>,
    cache_enabled: bool,
    capabilities: ServerCapabilities,
    client: Box<dyn RemoteClient>,
}

impl RemoteService {
    pub fn session() -> Option<Arc<RemoteService>> {
        INSTANCE.get().cloned()
    }

    #[instrument(skip(config))]
    pub async fn new(config: &RemoteConfig) -> miette::Result<Arc<RemoteService>> {
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
            client,
            cache_enabled: false,
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
                    "Remote service does not support SHA256 digests, disabling in moon"
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

    pub async fn is_action_cached(&self, digest: &Digest) -> miette::Result<bool> {
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

    pub async fn create_action_result(
        &self,
        digest: &Digest,
        operation: &Operation,
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

        match &operation.meta {
            OperationMeta::ProcessExecution(exec) | OperationMeta::TaskExecution(exec) => {
                let mut blobs = vec![];
                let mut stderr_index = -1;
                let mut stdout_index = -1;

                if let Some(stderr) = &exec.stderr {
                    stderr_index = blobs.len() as i8;
                    blobs.push(Blob::new(stderr.as_bytes().to_owned()));
                }

                if let Some(stdout) = &exec.stdout {
                    stdout_index = blobs.len() as i8;
                    blobs.push(Blob::new(stdout.as_bytes().to_owned()));
                }

                if !blobs.is_empty() {
                    let mut digests = self.client.batch_update_blobs(digest, blobs).await?;

                    if stderr_index >= 0 {
                        result.stderr_digest = digests
                            .get_mut(stderr_index as usize)
                            .and_then(|item| item.take());
                    }

                    if stdout_index >= 0 {
                        result.stdout_digest = digests
                            .get_mut(stdout_index as usize)
                            .and_then(|item| item.take());
                    }
                }

                result.exit_code = exec.exit_code.unwrap_or_default();
            }
            _ => {}
        };

        Ok(result)
    }

    pub async fn save_action(&self, digest: &Digest, operation: &Operation) -> miette::Result<()> {
        if !self.cache_enabled || operation.get_output().is_none() || operation.has_failed() {
            return Ok(());
        }

        debug!(
            hash = &digest.hash,
            "Caching {} action operation",
            color::muted_light(operation.label())
        );

        self.client
            .update_action_result(digest, self.create_action_result(digest, operation).await?)
            .await?;

        Ok(())
    }

    pub async fn save_action_with_outputs(
        &self,
        digest: &Digest,
        operation: &Operation,
        outputs: OutputDigests,
    ) -> miette::Result<()> {
        if !self.cache_enabled || operation.get_output().is_none() || operation.has_failed() {
            return Ok(());
        }

        debug!(
            hash = &digest.hash,
            "Caching {} action operation with outputs",
            color::muted_light(operation.label())
        );

        let mut result = self.create_action_result(digest, operation).await?;
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

        self.client.update_action_result(digest, result).await?;

        Ok(())
    }
}
