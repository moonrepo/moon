use crate::grpc_remote_client::GrpcRemoteClient;
use crate::remote_client::RemoteClient;
use bazel_remote_apis::build::bazel::remote::{
    asset::v1::Qualifier,
    execution::v2::{digest_function, ServerCapabilities},
};
use miette::IntoDiagnostic;
use moon_common::color;
use moon_config::RemoteConfig;
use moon_project::Project;
use moon_task::Task;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

static INSTANCE: OnceLock<Arc<RemoteService>> = OnceLock::new();

pub struct RemoteService {
    pub config: RemoteConfig,

    capabilities: ServerCapabilities,
    client: RwLock<Box<dyn RemoteClient>>,

    cache_enabled: bool,
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
            capabilities: client.load_capabilities().await?,
            config: config.to_owned(),
            client: RwLock::new(client),
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

    pub async fn upload_artifact(
        &self,
        project: &Project,
        task: &Task,
        hash: &str,
        bytes: Vec<u8>,
    ) -> miette::Result<()> {
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

        let mut client = self.client.write().await;

        let digest = match client.upload_blob(hash, bytes).await {
            Ok(digest) => digest,
            Err(error) => {
                warn!(
                    hash,
                    "Failed to upload artifact to remote storage: {}",
                    color::muted_light(error.to_string()),
                );

                return Ok(());
            }
        };

        let qualifiers = vec![
            Qualifier {
                name: "resource_type".into(),
                value: "application/gzip".into(),
            },
            Qualifier {
                name: "moon.task_target".into(),
                value: task.target.to_string(),
            },
        ];

        client.create_asset(digest, qualifiers).await?;

        Ok(())
    }

    pub fn download_artifact(&self) -> miette::Result<()> {
        Ok(())
    }
}
