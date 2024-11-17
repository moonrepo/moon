use crate::fs_digest::calculate_digests_for_outputs;
use crate::grpc_remote_client::GrpcRemoteClient;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::{
    asset::v1::Qualifier,
    execution::v2::{
        digest_function, ActionResult, Digest, ExecutedActionMetadata, ServerCapabilities,
    },
};
use miette::IntoDiagnostic;
use moon_action::{ActionStatus, Operation, OperationMeta};
use moon_common::color;
use moon_config::RemoteConfig;
use moon_project::Project;
use moon_task::Task;
use prost_types::Timestamp;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

static INSTANCE: OnceLock<Arc<RemoteService>> = OnceLock::new();

pub struct RemoteService {
    pub config: RemoteConfig,
    pub workspace_root: PathBuf,

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
            client,
            cache_enabled: false,
            workspace_root: workspace_root.to_path_buf(),
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

        if let Some(result) = self.client.get_action_result(digest.to_owned()).await? {
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
        operation: &Operation,
    ) -> miette::Result<ActionResult> {
        let mut result = ActionResult {
            execution_metadata: Some(ExecutedActionMetadata {
                worker: "moon".into(),
                execution_start_timestamp: None,
                execution_completed_timestamp: None,
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
                    blobs.push(stderr.as_bytes().to_owned());
                }

                if let Some(stdout) = &exec.stdout {
                    stdout_index = blobs.len() as i8;
                    blobs.push(stdout.as_bytes().to_owned());
                }

                if !blobs.is_empty() {
                    let mut digests = self.client.batch_update_blobs(blobs).await?;

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

    pub async fn cache_run_task_action(
        &self,
        digest: &Digest,
        operation: &Operation,
        task: &Task,
    ) -> miette::Result<()> {
        if !self.cache_enabled || operation.get_output().is_none() || operation.has_failed() {
            return Ok(());
        }

        let output_digests = calculate_digests_for_outputs(
            task.get_output_files(&self.workspace_root, true)?,
            &self.workspace_root,
        )?;

        let mut result = self.create_action_result(operation).await?;
        result.output_files = output_digests.files;
        result.output_symlinks = output_digests.symlinks;
        result.output_directories = output_digests.dirs;

        self.client
            .update_action_result(digest.to_owned(), result)
            .await?;

        Ok(())
    }

    pub async fn upload_artifact(
        &self,
        project: &Project,
        task: &Task,
        hash: &str,
        bytes: Vec<u8>,
    ) -> miette::Result<()> {
        return Ok(());

        if !self.cache_enabled {
            return Ok(());
        }

        if let Some(cap) = &self.capabilities.cache_capabilities {
            // 0 = no limit
            if cap.max_batch_total_size_bytes > 0
                && bytes.len() as i64 > cap.max_batch_total_size_bytes
            {
                debug!(
                    hash,
                    size = bytes.len(),
                    maz_size = cap.max_batch_total_size_bytes,
                    "Unable to upload artifact, as the blob size is larger than the maximum allowed by the remote server",
                );

                return Ok(());
            }
        }

        // let digest = match self.client.upload_blob(hash, bytes).await {
        //     Ok(digest) => digest,
        //     Err(error) => {
        //         warn!(
        //             hash,
        //             "Failed to upload artifact to remote storage: {}",
        //             color::muted_light(error.to_string()),
        //         );

        //         return Ok(());
        //     }
        // };

        Ok(())
    }

    pub fn download_artifact(&self) -> miette::Result<()> {
        Ok(())
    }
}
