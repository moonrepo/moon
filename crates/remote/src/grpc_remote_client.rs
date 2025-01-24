use crate::compression::*;
use crate::fs_digest::Blob;
use crate::grpc_tls::*;
use crate::remote_client::RemoteClient;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    action_cache_client::ActionCacheClient, batch_update_blobs_request,
    capabilities_client::CapabilitiesClient,
    content_addressable_storage_client::ContentAddressableStorageClient, digest_function,
    ActionResult, BatchReadBlobsRequest, BatchUpdateBlobsRequest, Digest, FindMissingBlobsRequest,
    GetActionResultRequest, GetCapabilitiesRequest, ServerCapabilities, UpdateActionResultRequest,
};
use miette::IntoDiagnostic;
use moon_common::color;
use moon_config::RemoteConfig;
use std::{env, path::Path, str::FromStr};
use tonic::{
    metadata::{KeyAndValueRef, MetadataKey, MetadataMap, MetadataValue},
    transport::{Channel, Endpoint},
    Code, Request, Status,
};
use tracing::{trace, warn};

fn map_transport_error(error: tonic::transport::Error) -> RemoteError {
    dbg!(&error);
    RemoteError::GrpcConnectFailed {
        error: Box::new(error),
    }
}

fn map_status_error(error: tonic::Status) -> RemoteError {
    dbg!(&error);
    RemoteError::GrpcCallFailed {
        error: Box::new(error),
    }
}

#[derive(Default)]
pub struct GrpcRemoteClient {
    channel: Option<Channel>,
    config: RemoteConfig,
    headers: MetadataMap,
}

impl GrpcRemoteClient {
    fn extract_headers(&mut self) -> miette::Result<bool> {
        let mut enabled = true;

        if let Some(auth) = &self.config.auth {
            for (key, value) in &auth.headers {
                self.headers.insert(
                    MetadataKey::from_str(key).into_diagnostic()?,
                    MetadataValue::from_str(value).into_diagnostic()?,
                );
            }

            if let Some(token_name) = &auth.token {
                let token = env::var(token_name).unwrap_or_default();

                if token.is_empty() {
                    enabled = false;

                    warn!(
                        "Auth token {} does not exist, unable to authorize for remote service",
                        color::property(token_name)
                    );
                } else {
                    self.headers.insert(
                        MetadataKey::from_str("Authorization").into_diagnostic()?,
                        MetadataValue::from_str(&format!("Bearer {token}")).into_diagnostic()?,
                    );
                }
            }
        }

        Ok(enabled)
    }

    fn inject_auth_headers(&self, mut req: Request<()>) -> Result<Request<()>, Status> {
        if self.headers.is_empty() {
            return Ok(req);
        }

        let headers = req.metadata_mut();

        for entry in self.headers.iter() {
            match entry {
                KeyAndValueRef::Ascii(key, value) => {
                    headers.insert(key.clone(), value.clone());
                }
                KeyAndValueRef::Binary(key, value) => {
                    headers.insert_bin(key.clone(), value.clone());
                }
            };
        }

        Ok(req)
    }
}

#[async_trait::async_trait]
impl RemoteClient for GrpcRemoteClient {
    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        workspace_root: &Path,
    ) -> miette::Result<bool> {
        let host = &config.host;

        trace!(
            instance = &config.cache.instance_name,
            "Connecting to gRPC host {} {}",
            color::url(host),
            if config.mtls.is_some() {
                "(with mTLS)"
            } else if config.tls.is_some() {
                "(with TLS)"
            } else if config.is_bearer_auth() {
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
        } else if config.is_secure_protocol() {
            endpoint = endpoint
                .tls_config(create_native_tls_config()?)
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
        let enabled = self.extract_headers()?;

        // We can't inject auth headers into this initial connection,
        // so defer the connection until a client is used
        if self.config.is_bearer_auth() {
            self.channel = Some(endpoint.connect_lazy());
        } else {
            self.channel = Some(endpoint.connect().await.map_err(map_transport_error)?);
        }

        Ok(enabled)
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
                }
                // If we hit an out of range error, the payload is larger than the grpc
                // limit, and will fail the entire pipeline. Instead of letting that
                // happen, let's just do a cache miss instead...
                else if matches!(status.code(), Code::OutOfRange) {
                    trace!(
                        hash = &digest.hash,
                        "Cache miss because the expected payload is too large"
                    );

                    Ok(None)
                } else {
                    dbg!("get_action_result", digest);
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
                    dbg!("update_action_result", digest);
                    Err(map_status_error(status).into())
                }
            }
        }
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L351
    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>> {
        let mut client = ContentAddressableStorageClient::with_interceptor(
            self.channel.clone().unwrap(),
            |req| self.inject_auth_headers(req),
        );

        match client
            .find_missing_blobs(FindMissingBlobsRequest {
                instance_name: self.config.cache.instance_name.clone(),
                blob_digests,
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
        {
            Ok(response) => Ok(response.into_inner().missing_blob_digests),
            Err(status) => Err(map_status_error(status).into()),
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
                    dbg!("batch_read_blobs", digest);
                    Err(map_status_error(status).into())
                };
            }
        };

        let mut blobs = vec![];
        let mut total_count = 0;

        for download in response.into_inner().responses {
            if let Some(status) = download.status {
                let code = Code::from_i32(status.code);

                if !matches!(code, Code::Ok | Code::NotFound) {
                    warn!(
                        hash = &digest.hash,
                        details = ?status.details,
                        code = ?code,
                        "Failed to download blob: {}",
                        if status.message.is_empty() {
                            code.to_string()
                        } else {
                            status.message
                        }
                    );
                }
            }

            if let Some(digest) = download.digest {
                blobs.push(Blob {
                    digest,
                    bytes: decompress_blob(
                        get_compression_from_code(download.compressor),
                        download.data,
                    )?,
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
                    dbg!("batch_update_blobs", digest);
                    Err(map_status_error(status).into())
                };
            }
        };

        let mut digests = vec![];
        let mut uploaded_count = 0;

        for upload in response.into_inner().responses {
            if let Some(status) = upload.status {
                let code = Code::from_i32(status.code);

                if !matches!(code, Code::Ok) {
                    warn!(
                        hash = &digest.hash,
                        details = ?status.details,
                        code = ?code,
                        "Failed to upload blob: {}",
                        if status.message.is_empty() {
                            code.to_string()
                        } else {
                            status.message
                        }
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
