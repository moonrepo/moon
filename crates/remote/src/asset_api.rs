use bazel_remote_apis::build::bazel::remote::asset::v1::fetch_client::FetchClient;
use bazel_remote_apis::build::bazel::remote::asset::v1::push_client::PushClient;
use bazel_remote_apis::build::bazel::remote::asset::v1::{PushBlobRequest, Qualifier};
use miette::IntoDiagnostic;
use moon_config::{RemoteCacheCompression, RemoteConfig};
use moon_project::Project;
use moon_task::Task;
use tokio::sync::RwLock;
use tonic::codec::CompressionEncoding;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};

pub struct Asset {
    fetch_client: RwLock<FetchClient<Channel>>,
    push_client: RwLock<PushClient<Channel>>,
}

impl Asset {
    pub async fn new(config: &RemoteConfig) -> miette::Result<Self> {
        let mut endpoint = Endpoint::from_static("http://[::1]:50051");

        // Support TLS connections
        if let Some(tls) = &config.tls {
            // let pem = std::fs::read_to_string(data_dir.join("tls/ca.pem"))?;
            // let ca = Certificate::from_pem(pem);

            // let tls_config = ClientTlsConfig::new()
            //     .ca_certificate(ca)
            //     .domain_name("example.com")
            //     .with_enabled_roots();
        }

        let channel = endpoint.connect().await.into_diagnostic()?;

        let mut fetch_client = FetchClient::new(channel.clone());
        let mut push_client = PushClient::new(channel);

        // Apply compression to blobs
        if let Some(compression) = config.cache.compression {
            let encoding = match compression {
                RemoteCacheCompression::Gzip => CompressionEncoding::Gzip,
            };

            fetch_client = fetch_client
                .accept_compressed(encoding)
                .send_compressed(encoding);

            push_client = push_client
                .accept_compressed(encoding)
                .send_compressed(encoding);
        }

        Ok(Self {
            fetch_client: RwLock::new(fetch_client),
            push_client: RwLock::new(push_client),
        })
    }

    pub fn create_push_blob_request(&self, project: &Project, task: &Task) -> PushBlobRequest {
        let mut request = PushBlobRequest::default();
        request.instance_name = "moon_task_outputs".into();
        request.uris = vec![]; // TODO
        request.qualifiers = vec![
            Qualifier {
                name: "resource_type".into(),
                value: "application/gzip".into(),
            },
            Qualifier {
                name: "moon.project_id".into(),
                value: project.id.to_string(),
            },
            Qualifier {
                name: "moon.project_source".into(),
                value: project.source.to_string(),
            },
            Qualifier {
                name: "moon.task_id".into(),
                value: task.id.to_string(),
            },
            Qualifier {
                name: "moon.task_target".into(),
                value: task.target.to_string(),
            },
        ];
        request.expire_at = None;
        request.blob_digest = None;
        request
    }

    pub async fn push_asset(&self, request: PushBlobRequest) -> miette::Result<()> {
        let response = self
            .push_client
            .write()
            .await
            .push_blob(request)
            .await
            .into_diagnostic()?;

        // TODO handle response

        Ok(())
    }
}
