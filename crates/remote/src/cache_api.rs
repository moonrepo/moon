use bazel_remote_apis::build::bazel::remote::asset::v1::fetch_client::FetchClient;
use bazel_remote_apis::build::bazel::remote::asset::v1::push_client::PushClient;
use bazel_remote_apis::build::bazel::remote::asset::v1::{PushBlobRequest, Qualifier};
use bazel_remote_apis::build::bazel::remote::execution::v2::batch_update_blobs_request::Request as BlobRequest;
use bazel_remote_apis::build::bazel::remote::execution::v2::content_addressable_storage_client::ContentAddressableStorageClient;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    compressor, digest_function, BatchUpdateBlobsRequest, Digest,
};
use moon_config::{RemoteCacheCompression, RemoteConfig};
use moon_project::Project;
use moon_task::Task;
use starbase_utils::fs;
use std::path::Path;
use tokio::sync::RwLock;
use tonic::codec::CompressionEncoding;
use tonic::transport::Channel;

const INSTANCE_NAME: &str = "moon_task_outputs";

pub struct Cache {
    cas_client: RwLock<ContentAddressableStorageClient<Channel>>,
    fetch_client: RwLock<FetchClient<Channel>>,
    push_client: RwLock<PushClient<Channel>>,
}

impl Cache {
    pub async fn new(channel: Channel, config: &RemoteConfig) -> miette::Result<Self> {
        let mut cas_client = ContentAddressableStorageClient::new(channel.clone());
        let mut fetch_client = FetchClient::new(channel.clone());
        let mut push_client = PushClient::new(channel);

        // Apply compression to blobs
        if let Some(compression) = config.cache.compression {
            let encoding = match compression {
                RemoteCacheCompression::Gzip => CompressionEncoding::Gzip,
            };

            cas_client = cas_client
                .accept_compressed(encoding)
                .send_compressed(encoding);

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
            .push_blob(PushBlobRequest {
                instance_name: INSTANCE_NAME.into(),
                blob_digest: Some(digest),
                qualifiers: self.create_qualifiers(task),
                ..Default::default()
            })
            .await
        {
            // TODO handle error
            panic!("{:?}", error);
        }

        // // TODO handle response

        Ok(())
    }

    fn create_qualifiers(&self, task: &Task) -> Vec<Qualifier> {
        vec![
            Qualifier {
                name: "resource_type".into(),
                value: "application/gzip".into(),
            },
            Qualifier {
                name: "moon.task_target".into(),
                value: task.target.to_string(),
            },
        ]
    }
}
