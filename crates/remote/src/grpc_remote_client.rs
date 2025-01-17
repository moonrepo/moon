use crate::compression::*;
use crate::fs_digest::Blob;
use crate::grpc_tls::*;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    action_cache_client::ActionCacheClient, batch_update_blobs_request,
    capabilities_client::CapabilitiesClient,
    content_addressable_storage_client::ContentAddressableStorageClient, digest_function,
    ActionResult, BatchReadBlobsRequest, BatchUpdateBlobsRequest, Digest, GetActionResultRequest,
    GetCapabilitiesRequest, ServerCapabilities, UpdateActionResultRequest,
};
use moon_common::color;
use moon_config::RemoteConfig;
use std::{env, error::Error, path::Path};
use tonic::{
    metadata::{MetadataKey, MetadataValue},
    transport::{Channel, Endpoint},
    Code, Request, Status,
};
use tracing::{trace, warn};

fn map_transport_error(error: tonic::transport::Error) -> RemoteError {
    RemoteError::GrpcConnectFailed {
        error: Box::new(error),
    }
}

fn map_status_error(error: tonic::Status) -> RemoteError {
    dbg!(&error);

    match error.source() {
        Some(src) => RemoteError::GrpcCallFailedViaSource {
            error: src.to_string(),
        },
        None => RemoteError::GrpcCallFailed {
            error: Box::new(error),
        },
    }
}

#[derive(Default)]
pub struct GrpcRemoteClient {
    channel: Option<Channel>,
    config: RemoteConfig,
}

impl GrpcRemoteClient {
    fn inject_auth_headers(&self, mut req: Request<()>) -> Result<Request<()>, Status> {
        if self.config.mtls.is_some() || self.config.tls.is_some() {
            return Ok(req);
        }

        if let Some(auth) = &self.config.auth {
            let headers = req.metadata_mut();

            for (key, value) in &auth.headers {
                headers.insert(
                    MetadataKey::from_bytes(key.as_bytes()).unwrap(),
                    MetadataValue::try_from(value).unwrap(),
                );
            }

            if let Some(token_name) = &auth.token {
                let token = env::var(token_name).unwrap_or_default();

                if token.is_empty() {
                    warn!(
                        "Remote service auth token {} does not exist, unable to authorize",
                        color::property(token_name)
                    );
                } else {
                    headers.insert(
                        "Authorization",
                        MetadataValue::try_from(format!("Bearer {token}")).unwrap(),
                    );
                }
            }
        }

        dbg!(&req);

        Ok(req)
    }
}

#[async_trait::async_trait]
impl RemoteClient for GrpcRemoteClient {
    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        workspace_root: &Path,
    ) -> miette::Result<()> {
        let host = &config.host;

        trace!(
            instance = &config.cache.instance_name,
            "Connecting to gRPC host {} {}",
            color::url(host),
            if config.mtls.is_some() {
                "(with mTLS)"
            } else if config.tls.is_some() {
                "(with TLS)"
            } else if config.auth.is_some() {
                "(with auth)"
            } else {
                "(insecure)"
            }
        );

        // Although we use a grpc(s) protocol for the host,
        // tonic only supports http(s), so change it
        let url = if let Some(suffix) = host.strip_prefix("grpc") {
            format!("http{suffix}")
        } else {
            host.to_owned()
        };

        let mut endpoint = Endpoint::from_shared(url)
            .map_err(map_transport_error)?
            .user_agent("moon")
            .map_err(map_transport_error)?
            .keep_alive_while_idle(true);

        if let Some(mtls) = &config.mtls {
            endpoint = endpoint
                .tls_config(create_mtls_config(mtls, workspace_root)?)
                .map_err(map_transport_error)?;
        } else if let Some(tls) = &config.tls {
            endpoint = endpoint
                .tls_config(create_tls_config(tls, workspace_root)?)
                .map_err(map_transport_error)?;
        }

        if config.is_localhost() {
            endpoint = endpoint.origin(
                format!(
                    "{}://localhost",
                    if config.is_secure() { "https" } else { "http" }
                )
                .parse()
                .unwrap(),
            );
        }

        self.config = config.to_owned();

        // We can't inject auth headers into this initial connection,
        // so defer the connection until a client is used
        if self.config.auth.is_some() {
            self.channel = Some(endpoint.connect_lazy());
        } else {
            self.channel = Some(endpoint.connect().await.map_err(map_transport_error)?);
        }

        Ok(())
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L452
    async fn load_capabilities(&self) -> miette::Result<ServerCapabilities> {
        let mut client =
            CapabilitiesClient::with_interceptor(self.channel.clone().unwrap(), |req| {
                self.inject_auth_headers(req)
            });

        trace!("Loading remote execution API capabilities from gRPC server");

        let response = client
            .get_capabilities(GetCapabilitiesRequest {
                instance_name: self.config.cache.instance_name.clone(),
            })
            .await
            .map_err(map_status_error)?;

        Ok(response.into_inner())
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L170
    async fn get_action_result(&self, digest: &Digest) -> miette::Result<Option<ActionResult>> {
        let mut client =
            ActionCacheClient::with_interceptor(self.channel.clone().unwrap(), |req| {
                self.inject_auth_headers(req)
            });

        trace!(hash = &digest.hash, "Checking for a cached action result");

        match client
            .get_action_result(GetActionResultRequest {
                instance_name: self.config.cache.instance_name.clone(),
                action_digest: Some(digest.to_owned()),
                inline_stderr: true,
                inline_stdout: true,
                digest_function: digest_function::Value::Sha256 as i32,
                ..Default::default()
            })
            .await
        {
            Ok(response) => {
                let result = response.into_inner();

                trace!(
                    hash = &digest.hash,
                    files = result.output_files.len(),
                    links = result.output_symlinks.len(),
                    dirs = result.output_directories.len(),
                    exit_code = result.exit_code,
                    "Cache hit on action result"
                );

                Ok(Some(result))
            }
            Err(status) => {
                if matches!(status.code(), Code::NotFound) {
                    trace!(hash = &digest.hash, "Cache miss on action result");

                    Ok(None)
                } else {
                    Err(map_status_error(status).into())
                }
            }
        }
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L193
    async fn update_action_result(
        &self,
        digest: &Digest,
        result: ActionResult,
    ) -> miette::Result<Option<ActionResult>> {
        let mut client =
            ActionCacheClient::with_interceptor(self.channel.clone().unwrap(), |req| {
                self.inject_auth_headers(req)
            });

        trace!(
            hash = &digest.hash,
            files = result.output_files.len(),
            links = result.output_symlinks.len(),
            dirs = result.output_directories.len(),
            exit_code = result.exit_code,
            "Caching action result"
        );

        match client
            .update_action_result(UpdateActionResultRequest {
                instance_name: self.config.cache.instance_name.clone(),
                action_digest: Some(digest.to_owned()),
                action_result: Some(result),
                digest_function: digest_function::Value::Sha256 as i32,
                ..Default::default()
            })
            .await
        {
            Ok(response) => {
                trace!(hash = &digest.hash, "Cached action result");

                Ok(Some(response.into_inner()))
            }
            Err(status) => {
                let code = status.code();

                if matches!(code, Code::InvalidArgument | Code::FailedPrecondition) {
                    warn!(
                        code = ?code,
                        "Failed to cache action result: {}",
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
                    Err(map_status_error(status).into())
                }
            }
        }
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L403
    async fn batch_read_blobs(
        &self,
        digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Blob>> {
        let mut client = ContentAddressableStorageClient::with_interceptor(
            self.channel.clone().unwrap(),
            |req| self.inject_auth_headers(req),
        );

        trace!(
            hash = &digest.hash,
            compression = self.config.cache.compression.to_string(),
            "Downloading {} output blobs",
            blob_digests.len()
        );

        let response = match client
            .batch_read_blobs(BatchReadBlobsRequest {
                acceptable_compressors: get_acceptable_compressors(self.config.cache.compression),
                instance_name: self.config.cache.instance_name.clone(),
                digests: blob_digests,
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
        {
            Ok(res) => res,
            Err(status) => {
                return if matches!(status.code(), Code::InvalidArgument) {
                    warn!(
                        hash = &digest.hash,
                        "Attempted to download more blobs than the allowed limit"
                    );

                    Ok(vec![])
                } else {
                    Err(map_status_error(status).into())
                };
            }
        };

        let mut blobs = vec![];
        let mut total_count = 0;

        for download in response.into_inner().responses {
            if let Some(status) = download.status {
                if status.code != 0 {
                    warn!(
                        details = ?status.details,
                        "Failed to download blob: {}",
                        status.message
                    );
                }
            }

            if let Some(digest) = download.digest {
                blobs.push(Blob {
                    digest,
                    bytes: decompress_blob(self.config.cache.compression, download.data)?,
                });
            }

            total_count += 1;
        }

        trace!(
            hash = &digest.hash,
            "Downloaded {} of {} output blobs",
            blobs.len(),
            total_count
        );

        Ok(blobs)
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L379
    async fn batch_update_blobs(
        &self,
        digest: &Digest,
        blobs: Vec<Blob>,
    ) -> miette::Result<Vec<Option<Digest>>> {
        let mut client = ContentAddressableStorageClient::with_interceptor(
            self.channel.clone().unwrap(),
            |req| self.inject_auth_headers(req),
        );

        trace!(
            hash = &digest.hash,
            compression = self.config.cache.compression.to_string(),
            "Uploading {} output blobs",
            blobs.len()
        );

        let mut requests = vec![];

        for blob in blobs {
            requests.push(batch_update_blobs_request::Request {
                digest: Some(blob.digest),
                data: compress_blob(self.config.cache.compression, blob.bytes)?,
                compressor: get_compressor(self.config.cache.compression),
            });
        }

        let response = match client
            .batch_update_blobs(BatchUpdateBlobsRequest {
                instance_name: self.config.cache.instance_name.clone(),
                requests,
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
        {
            Ok(res) => res,
            Err(status) => {
                let code = status.code();

                return if matches!(code, Code::InvalidArgument) {
                    warn!(
                        hash = &digest.hash,
                        "Attempted to upload more blobs than the allowed limit"
                    );

                    Ok(vec![])
                } else if matches!(code, Code::ResourceExhausted) {
                    warn!(
                        code = ?code,
                        "Remote service exhausted resource: {}",
                        status.message()
                    );

                    Ok(vec![])
                } else {
                    Err(map_status_error(status).into())
                };
            }
        };

        let mut digests = vec![];
        let mut uploaded_count = 0;

        for upload in response.into_inner().responses {
            if let Some(status) = upload.status {
                if status.code != 0 {
                    warn!(
                        details = ?status.details,
                        "Failed to upload blob: {}",
                        status.message
                    );
                }
            }

            if upload.digest.is_some() {
                uploaded_count += 1;
            }

            digests.push(upload.digest);
        }

        trace!(
            hash = &digest.hash,
            "Uploaded {} of {} output blobs",
            uploaded_count,
            digests.len()
        );

        Ok(digests)
    }
}
