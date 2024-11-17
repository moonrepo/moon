use crate::fs_digest::create_digest;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    action_cache_client::ActionCacheClient, batch_update_blobs_request,
    capabilities_client::CapabilitiesClient, compressor,
    content_addressable_storage_client::ContentAddressableStorageClient, digest_function,
    ActionResult, BatchUpdateBlobsRequest, Digest, GetActionResultRequest, GetCapabilitiesRequest,
    ServerCapabilities, UpdateActionResultRequest,
};
use miette::IntoDiagnostic;
use moon_config::RemoteConfig;
use tonic::{
    transport::{Channel, Endpoint},
    Code,
};
use tracing::warn;
// use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};

const INSTANCE_NAME: &str = "moon_task_outputs";

#[derive(Default)]
pub struct GrpcRemoteClient {
    channel: Option<Channel>,
}

#[async_trait::async_trait]
impl RemoteClient for GrpcRemoteClient {
    async fn connect_to_host(&mut self, host: &str, _config: &RemoteConfig) -> miette::Result<()> {
        let endpoint = Endpoint::from_shared(host.to_owned()).into_diagnostic()?;

        self.channel = Some(endpoint.connect().await.into_diagnostic()?);

        Ok(())
    }

    async fn load_capabilities(&self) -> miette::Result<ServerCapabilities> {
        let mut client = CapabilitiesClient::new(self.channel.clone().unwrap());

        let response = client
            .get_capabilities(GetCapabilitiesRequest {
                instance_name: INSTANCE_NAME.into(),
            })
            .await
            .expect("TODO response");

        dbg!("load_capabilities", &response);

        Ok(response.into_inner())
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L170
    async fn get_action_result(&self, digest: Digest) -> miette::Result<Option<ActionResult>> {
        let mut client = ActionCacheClient::new(self.channel.clone().unwrap());

        match client
            .get_action_result(GetActionResultRequest {
                instance_name: INSTANCE_NAME.into(),
                action_digest: Some(digest),
                inline_stderr: true,
                inline_stdout: true,
                digest_function: digest_function::Value::Sha256 as i32,
                ..Default::default()
            })
            .await
        {
            Ok(response) => Ok(Some(response.into_inner())),
            Err(status) => {
                if matches!(status.code(), Code::NotFound) {
                    Ok(None)
                } else {
                    Err(RemoteError::Tonic(Box::new(status)).into())
                }
            }
        }
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L193
    async fn update_action_result(
        &self,
        digest: Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>> {
        let mut client = ActionCacheClient::new(self.channel.clone().unwrap());

        match client
            .update_action_result(UpdateActionResultRequest {
                instance_name: INSTANCE_NAME.into(),
                action_digest: Some(digest),
                action_result: Some(result),
                digest_function: digest_function::Value::Sha256 as i32,
                ..Default::default()
            })
            .await
        {
            Ok(response) => Ok(Some(response.into_inner())),
            Err(status) => {
                let code = status.code();

                if matches!(code, Code::InvalidArgument | Code::FailedPrecondition) {
                    warn!(
                        code = ?code,
                        "Failed to update action result: {}",
                        status.message()
                    );

                    Ok(None)
                } else if matches!(code, Code::ResourceExhausted) {
                    warn!(
                        code = ?code,
                        "Remote service is out of storage space: {}",
                        status.message()
                    );

                    Ok(None)
                } else {
                    Err(RemoteError::Tonic(Box::new(status)).into())
                }
            }
        }
    }

    async fn batch_update_blobs(&self, blobs: Vec<Vec<u8>>) -> miette::Result<Vec<Option<Digest>>> {
        let mut client = ContentAddressableStorageClient::new(self.channel.clone().unwrap());

        let response = client
            .batch_update_blobs(BatchUpdateBlobsRequest {
                instance_name: INSTANCE_NAME.into(),
                requests: blobs
                    .into_iter()
                    .map(|blob| batch_update_blobs_request::Request {
                        digest: Some(create_digest(&blob)),
                        data: blob,
                        compressor: compressor::Value::Identity as i32,
                    })
                    .collect(),
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
            .expect("TODO response");

        dbg!("batch_update_blobs", &response);

        let mut digests = vec![];

        for upload in response.into_inner().responses {
            if let Some(status) = upload.status {
                warn!(
                    details = ?status.details,
                    "Failed to upload blob: {}",
                    status.message
                );
            }

            digests.push(upload.digest);
        }

        Ok(digests)
    }
}
