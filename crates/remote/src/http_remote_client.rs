use crate::compression::*;
use crate::fs_digest::Blob;
use crate::http_endpoints::*;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    action_cache_client::ActionCacheClient, batch_update_blobs_request,
    capabilities_client::CapabilitiesClient, compressor,
    content_addressable_storage_client::ContentAddressableStorageClient, digest_function,
    ActionCacheUpdateCapabilities, ActionResult, BatchReadBlobsRequest, BatchUpdateBlobsRequest,
    CacheCapabilities, Digest, ExecutionCapabilities, GetActionResultRequest,
    GetCapabilitiesRequest, ServerCapabilities, UpdateActionResultRequest,
};
use moon_common::color;
use moon_config::{RemoteCompression, RemoteConfig};
use reqwest::Client;
use std::{error::Error, path::Path, sync::OnceLock};
use tonic::{
    transport::{Channel, Endpoint, Server},
    Code,
};
use tracing::{trace, warn};

#[derive(Default)]
pub struct HttpRemoteClient {
    client: OnceLock<Client>,
    compression: RemoteCompression,
    host: String,
    instance_name: String,
}

impl HttpRemoteClient {
    fn get_client(&self) -> &Client {
        self.client.get_or_init(|| Client::new())
    }
}

#[async_trait::async_trait]
impl RemoteClient for HttpRemoteClient {
    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        _workspace_root: &Path,
    ) -> miette::Result<()> {
        self.compression = config.cache.compression;
        self.host = config.host.clone();
        self.instance_name = config.cache.instance_name.clone();

        // Ignore errors since this endpoint is non-standard
        if let Ok(response) = self
            .get_client()
            .get(format!("{}/status", self.host))
            .send()
            .await
        {
            let status = response.status();
            let code = status.as_u16();

            if !status.is_success() && code != 404 {
                return Err(RemoteError::HttpConnectFailed {
                    code,
                    reason: status
                        .canonical_reason()
                        .map(|reason| reason.to_owned())
                        .unwrap_or_else(|| String::from("Unknown")),
                }
                .into());
            }
        }

        Ok(())
    }

    // HTTP API doesn't support capabilities, so we need to fake this
    // based on what `bazel-remote` supports
    async fn load_capabilities(&self) -> miette::Result<ServerCapabilities> {
        let digest_functions = vec![digest_function::Value::Sha256 as i32];
        let compressors = get_acceptable_compressors(self.compression);

        Ok(ServerCapabilities {
            cache_capabilities: Some(CacheCapabilities {
                digest_functions: digest_functions.clone(),
                action_cache_update_capabilities: Some(ActionCacheUpdateCapabilities {
                    update_enabled: true,
                }),
                supported_compressors: compressors.clone(),
                supported_batch_update_compressors: compressors,
                ..Default::default()
            }),
            execution_capabilities: Some(ExecutionCapabilities {
                digest_functions,
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    async fn get_action_result(&self, digest: &Digest) -> miette::Result<Option<ActionResult>> {
        trace!(hash = &digest.hash, "Checking for a cached action result");

        dbg!(&digest);

        match self
            .get_client()
            .get(format!(
                "{}/{}/cas/{}",
                self.host, self.instance_name, digest.hash
            ))
            .send()
            .await
        {
            Ok(response) => {
                dbg!(&response);
                // let result = response.into_inner();

                // trace!(
                //     hash = &digest.hash,
                //     files = result.output_files.len(),
                //     links = result.output_symlinks.len(),
                //     dirs = result.output_directories.len(),
                //     exit_code = result.exit_code,
                //     "Cache hit on action result"
                // );

                // Ok(Some(result))
                Ok(None)
            }
            Err(error) => {
                dbg!(&error);
                // if matches!(status.code(), Code::NotFound) {
                //     trace!(hash = &digest.hash, "Cache miss on action result");

                //     Ok(None)
                // } else {
                //     Err(map_status_error(status).into())
                // }
                Ok(None)
            }
        }
    }

    async fn update_action_result(
        &self,
        digest: &Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>> {
        Ok(None)
    }

    async fn batch_read_blobs(
        &self,
        digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Blob>> {
        Ok(vec![])
    }

    async fn batch_update_blobs(
        &self,
        digest: &Digest,
        blobs: Vec<Blob>,
    ) -> miette::Result<Vec<Option<Digest>>> {
        Ok(vec![])
    }
}
