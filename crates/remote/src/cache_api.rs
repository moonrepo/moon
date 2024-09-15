use std::path::Path;

use bazel_remote_apis::build::bazel::remote::asset::v1::fetch_client::FetchClient;
use bazel_remote_apis::build::bazel::remote::asset::v1::push_client::PushClient;
use bazel_remote_apis::build::bazel::remote::asset::v1::{PushBlobRequest, Qualifier};
use bazel_remote_apis::build::bazel::remote::execution::v2::batch_update_blobs_request::Request as BlobRequest;
use bazel_remote_apis::build::bazel::remote::execution::v2::content_addressable_storage_client::ContentAddressableStorageClient;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    compressor, digest_function, BatchUpdateBlobsRequest, Digest,
};
use miette::IntoDiagnostic;
use moon_config::{RemoteCacheCompression, RemoteConfig};
use moon_project::Project;
use moon_task::Task;
use starbase_utils::fs;
use tokio::sync::RwLock;
use tonic::codec::CompressionEncoding;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};

const INSTANCE_NAME: &str = "moon_task_outputs";

pub struct Cache {
    cas_client: RwLock<ContentAddressableStorageClient<Channel>>,
    fetch_client: RwLock<FetchClient<Channel>>,
    push_client: RwLock<PushClient<Channel>>,
}

impl Cache {
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

        let mut cas_client = ContentAddressableStorageClient::new(channel.clone());
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
            cas_client: RwLock::new(cas_client),
            fetch_client: RwLock::new(fetch_client),
            push_client: RwLock::new(push_client),
        })
    }

    pub fn create_digest(&self, hash: &str, src_path: &Path) -> miette::Result<Digest> {
        Ok(Digest {
            hash: hash.into(),
            size_bytes: fs::metadata(src_path).map_or(0, |meta| meta.len()) as i64,
        })
    }

    pub async fn upload_artifact(
        &self,
        project: &Project,
        task: &Task,
        hash: &str,
        path: &Path,
    ) -> miette::Result<()> {
        let digest = self.create_digest(hash, path)?;

        // Upload the blob to the CAS
        if let Err(error) = self
            .cas_client
            .write()
            .await
            .batch_update_blobs(BatchUpdateBlobsRequest {
                instance_name: INSTANCE_NAME.into(),
                requests: vec![BlobRequest {
                    digest: Some(digest.clone()),
                    data: fs::read_file_bytes(path)?,
                    compressor: compressor::Value::Identity as i32,
                }],
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
        {
            // TODO handle error
            panic!("{:?}", error);
        }

        // Make the CAS blob available as an asset
        if let Err(error) = self
            .push_client
            .write()
            .await
            .push_blob(self.create_push_blob_request(project, task, digest))
            .await
        {
            // TODO handle error
            panic!("{:?}", error);
        }

        // // TODO handle response

        Ok(())
    }

    pub fn create_push_blob_request(
        &self,
        project: &Project,
        task: &Task,
        digest: Digest,
    ) -> PushBlobRequest {
        let mut request = PushBlobRequest::default();
        request.instance_name = INSTANCE_NAME.into();
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
        request.expire_at = None; // Add task option
        request.blob_digest = Some(digest);
        request
    }
}
