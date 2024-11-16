use crate::cache_api::Cache;
use crate::grpc_remote_client::GrpcRemoteClient;
use crate::remote_client::RemoteClient;
use miette::IntoDiagnostic;
use moon_config::RemoteConfig;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

static INSTANCE: OnceLock<Arc<RemoteService>> = OnceLock::new();

pub struct RemoteService {
    pub config: RemoteConfig,
    client: RwLock<Box<dyn RemoteClient>>,
}

impl RemoteService {
    pub fn session() -> Option<Arc<RemoteService>> {
        INSTANCE.get().cloned()
    }

    pub fn new(config: &RemoteConfig) -> miette::Result<Arc<RemoteService>> {
        let client = if config.host.starts_with("http://") || config.host.starts_with("https://") {
            todo!("TODO http client");
        } else if config.host.starts_with("grpc://") || config.host.starts_with("grpcs://") {
            Box::new(GrpcRemoteClient::default())
        } else {
            todo!("Handle error")
        };

        let service = Arc::new(Self {
            config: config.to_owned(),
            client: RwLock::new(client),
        });

        let _ = INSTANCE.set(Arc::clone(&service));

        Ok(service)
    }

    pub async fn connect(&self) -> miette::Result<()> {
        let mut client = self.client.write().await;

        client
            .connect_to_host(&self.config.host, &self.config)
            .await?;

        Ok(())
    }
}
