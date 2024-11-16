use std::sync::OnceLock;

use crate::remote_client::RemoteClient;
use bazel_remote_apis::build::bazel::remote::{
    asset::v1::{push_client::PushClient, PushBlobRequest, Qualifier},
    execution::v2::{
        batch_update_blobs_request, capabilities_client::CapabilitiesClient, compressor,
        content_addressable_storage_client::ContentAddressableStorageClient, digest_function,
        BatchUpdateBlobsRequest, Digest, GetCapabilitiesRequest, ServerCapabilities,
    },
};
use miette::IntoDiagnostic;
use moon_config::RemoteConfig;
use moon_project::Project;
use moon_task::Task;
use tonic::transport::{Channel, Endpoint};
// use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};

const INSTANCE_NAME: &str = "moon_task_outputs";

#[derive(Default)]
pub struct GrpcRemoteClient {
    channel: Option<Channel>,
}

#[async_trait::async_trait]
impl RemoteClient for GrpcRemoteClient {
    async fn connect_to_host(&mut self, host: &str, config: &RemoteConfig) -> miette::Result<()> {
        let mut endpoint = Endpoint::from_shared(host.to_owned()).into_diagnostic()?;

        dbg!("connect_to_host", &endpoint);

        self.channel = Some(endpoint.connect().await.into_diagnostic()?);

        dbg!("connect_to_host", self.channel.as_ref());

        Ok(())
    }

    async fn load_capabilities(&mut self) -> miette::Result<ServerCapabilities> {
        let mut client = CapabilitiesClient::new(self.channel.clone().unwrap());

        let response = client
            .get_capabilities(GetCapabilitiesRequest {
                instance_name: INSTANCE_NAME.into(),
            })
            .await
            .expect("TODO response");

        dbg!(&response);

        Ok(response.into_inner())
    }

    async fn upload_blob(&self, hash: &str, bytes: Vec<u8>) -> miette::Result<Digest> {
        let digest = Digest {
            hash: hash.to_owned(),
            size_bytes: bytes.len() as i64,
        };

        let mut client = ContentAddressableStorageClient::new(self.channel.clone().unwrap());

        let response = client
            .batch_update_blobs(BatchUpdateBlobsRequest {
                instance_name: INSTANCE_NAME.into(),
                requests: vec![batch_update_blobs_request::Request {
                    digest: Some(digest.clone()),
                    data: bytes,
                    compressor: compressor::Value::Identity as i32,
                }],
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
            .expect("TODO response");

        dbg!("upload_artifact", hash, response);

        Ok(digest)
    }

    async fn create_asset(
        &self,
        digest: Digest,
        qualifiers: Vec<Qualifier>,
    ) -> miette::Result<Digest> {
        let mut client = PushClient::new(self.channel.clone().unwrap());

        let response = client
            .push_blob(PushBlobRequest {
                instance_name: INSTANCE_NAME.into(),
                blob_digest: Some(digest.clone()),
                digest_function: digest_function::Value::Sha256 as i32,
                qualifiers,
                ..Default::default()
            })
            .await
            .expect("TODO response");

        dbg!("create_asset", &digest, &response);

        Ok(digest)
    }
}
