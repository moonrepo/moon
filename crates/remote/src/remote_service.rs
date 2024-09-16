use crate::cache_api::Cache;
use miette::IntoDiagnostic;
use moon_config::RemoteConfig;
use std::sync::{Arc, OnceLock};
use tonic::transport::Endpoint;
// use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};

static INSTANCE: OnceLock<Arc<RemoteService>> = OnceLock::new();

pub struct RemoteService {
    pub cache: Cache,
    pub config: RemoteConfig,
}

impl RemoteService {
    pub fn session() -> Option<Arc<RemoteService>> {
        INSTANCE.get().cloned()
    }

    pub async fn new(config: &RemoteConfig) -> miette::Result<Arc<RemoteService>> {
        let endpoint = Endpoint::from_shared(config.host.clone()).into_diagnostic()?;

        // Support TLS connections
        // if let Some(tls) = &config.tls {
        //     let pem = std::fs::read_to_string(data_dir.join("tls/ca.pem"))?;
        //     let ca = Certificate::from_pem(pem);

        //     let tls_config = ClientTlsConfig::new()
        //         .ca_certificate(ca)
        //         .domain_name("example.com")
        //         .with_enabled_roots();
        // }

        let channel = endpoint.connect().await.into_diagnostic()?;

        let service = Arc::new(Self {
            cache: Cache::new(channel, config).await?,
            config: config.to_owned(),
        });

        let _ = INSTANCE.set(Arc::clone(&service));

        Ok(service)
    }
}
