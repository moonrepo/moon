use crate::blob::*;
use crate::grpc_services::*;
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
use bazel_remote_apis::google::bytestream::{byte_stream_client::ByteStreamClient, WriteRequest};
use http::header::HeaderMap;
use moon_common::color;
use moon_config::RemoteConfig;
use starbase_utils::env::bool_var;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::io::ReaderStream;
use tonic::{
    transport::{Channel, Endpoint},
    Code, Request,
};
use tower::{limit::ConcurrencyLimit, timeout::Timeout, ServiceBuilder};
use tracing::{debug, error, trace, warn};

type LayeredService = Timeout<ConcurrencyLimit<RequestHeaders<Channel>>>;

#[derive(Default)]
pub struct GrpcRemoteClient {
    channel: Option<Channel>,
    config: RemoteConfig,
    debug: bool,
    uuid: OnceLock<uuid::Uuid>,

    ac_client: OnceLock<ActionCacheClient<LayeredService>>,
    bs_client: OnceLock<ByteStreamClient<LayeredService>>,
    cap_client: OnceLock<CapabilitiesClient<LayeredService>>,
    cas_client: OnceLock<ContentAddressableStorageClient<LayeredService>>,
}

impl GrpcRemoteClient {
    fn create_clients(&mut self, headers: HeaderMap) {
        let service: LayeredService = ServiceBuilder::new()
            .timeout(Duration::from_secs(60 * 60))
            .concurrency_limit(150)
            .layer(RequestHeadersLayer::new(headers))
            .service(self.channel.clone().unwrap());

        let _ = self.ac_client.set(ActionCacheClient::new(service.clone()));

        let _ = self.bs_client.set(ByteStreamClient::new(service.clone()));

        let _ = self
            .cap_client
            .set(CapabilitiesClient::new(service.clone()));

        let _ = self
            .cas_client
            .set(ContentAddressableStorageClient::new(service));
    }

    fn get_ac_client(&self) -> ActionCacheClient<LayeredService> {
        self.ac_client.get().unwrap().clone()
    }

    fn get_bs_client(&self) -> ByteStreamClient<LayeredService> {
        self.bs_client.get().unwrap().clone()
    }

    fn get_cap_client(&self) -> CapabilitiesClient<LayeredService> {
        self.cap_client.get().unwrap().clone()
    }

    fn get_cas_client(&self) -> ContentAddressableStorageClient<LayeredService> {
        self.cas_client.get().unwrap().clone()
    }

    fn get_uuid(&self) -> &uuid::Uuid {
        self.uuid.get_or_init(uuid::Uuid::new_v4)
    }

    // TODO compression
    fn get_resource_name(&self, digest: &Digest) -> String {
        format!(
            "{}/uploads/{}/blobs/{}/{}",
            self.config.cache.instance_name,
            self.get_uuid(),
            digest.hash,
            digest.size_bytes,
        )
    }

    fn map_status_error(&self, method: &str, error: tonic::Status) -> RemoteError {
        if self.debug {
            error!("{method}: {:#?}", error);
        }

        RemoteError::GrpcCallFailed {
            error: Box::new(error),
        }
    }

    fn map_transport_error(&self, method: &str, error: tonic::transport::Error) -> RemoteError {
        if self.debug {
            error!("{method}: {:#?}", error);
        }

        RemoteError::GrpcConnectFailed {
            error: Box::new(error),
        }
    }
}

#[async_trait::async_trait]
impl RemoteClient for GrpcRemoteClient {
    async fn connect_to_host(
        &mut self,
        config: &RemoteConfig,
        workspace_root: &Path,
    ) -> miette::Result<bool> {
        debug!(
            instance = &config.cache.instance_name,
            "Connecting to gRPC host {} {}",
            color::url(&config.host),
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

        self.debug = bool_var("MOON_DEBUG_REMOTE");
        self.config = config.to_owned();

        // Extract headers and abort early if not enabled
        let Some(headers) = self.extract_headers(config)? else {
            return Ok(false);
        };

        // Although we use a grpc(s) protocol for the host,
        // tonic only supports http(s), so change it
        let url = if let Some(suffix) = self.config.host.strip_prefix("grpc") {
            format!("http{suffix}")
        } else {
            self.config.host.to_owned()
        };

        let mut endpoint = Endpoint::from_shared(url)
            .map_err(|error| self.map_transport_error("host", error))?
            .user_agent("moon")
            .map_err(|error| self.map_transport_error("user_agent", error))?
            .keep_alive_while_idle(true);

        if let Some(mtls) = &config.mtls {
            endpoint = endpoint
                .tls_config(create_mtls_config(mtls, workspace_root)?)
                .map_err(|error| self.map_transport_error("tls", error))?;
        } else if let Some(tls) = &config.tls {
            endpoint = endpoint
                .tls_config(create_tls_config(tls, workspace_root)?)
                .map_err(|error| self.map_transport_error("mtls", error))?;
        } else if config.is_secure_protocol() {
            endpoint = endpoint
                .tls_config(create_native_tls_config()?)
                .map_err(|error| self.map_transport_error("auth", error))?;
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

        // We can't inject auth headers into this initial connection,
        // so defer the connection until a client is used
        if self.config.is_bearer_auth() {
            self.channel = Some(endpoint.connect_lazy());
        } else {
            self.channel = Some(
                endpoint
                    .connect()
                    .await
                    .map_err(|error| self.map_transport_error("connect_to_host", error))?,
            );
        }

        self.create_clients(headers);

        Ok(true)
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L452
    async fn load_capabilities(&self) -> miette::Result<ServerCapabilities> {
        trace!("Loading remote execution API capabilities from gRPC server");

        let response = self
            .get_cap_client()
            .get_capabilities(GetCapabilitiesRequest {
                instance_name: self.config.cache.instance_name.clone(),
            })
            .await
            .map_err(|error| self.map_status_error("load_capabilities", error))?;

        Ok(response.into_inner())
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L170
    async fn get_action_result(&self, digest: &Digest) -> miette::Result<Option<ActionResult>> {
        trace!(hash = &digest.hash, "Checking for a cached action result");

        match self
            .get_ac_client()
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
                    Err(self.map_status_error("get_action_result", status).into())
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
        trace!(
            hash = &digest.hash,
            files = result.output_files.len(),
            links = result.output_symlinks.len(),
            dirs = result.output_directories.len(),
            exit_code = result.exit_code,
            "Caching action result"
        );

        match self
            .get_ac_client()
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
                        hash = &digest.hash,
                        code = ?code,
                        "Failed to cache action result: {}",
                        color::muted_light(status.message()),
                    );

                    Ok(None)
                } else if matches!(code, Code::ResourceExhausted) {
                    warn!(
                        hash = &digest.hash,
                        code = ?code,
                        "Remote service is out of storage space: {}",
                        color::muted_light(status.message()),
                    );

                    Ok(None)
                } else {
                    Err(self.map_status_error("update_action_result", status).into())
                }
            }
        }
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L351
    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>> {
        match self
            .get_cas_client()
            .find_missing_blobs(FindMissingBlobsRequest {
                instance_name: self.config.cache.instance_name.clone(),
                blob_digests,
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
        {
            Ok(response) => Ok(response.into_inner().missing_blob_digests),
            Err(status) => Err(self.map_status_error("find_missing_blobs", status).into()),
        }
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L403
    async fn batch_read_blobs(
        &self,
        digest: &Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Blob>> {
        trace!(
            hash = &digest.hash,
            compression = self.config.cache.compression.to_string(),
            "Downloading {} output blobs",
            blob_digests.len()
        );

        let response = match self
            .get_cas_client()
            .batch_read_blobs(BatchReadBlobsRequest {
                acceptable_compressors: get_acceptable_compressors(self.config.cache.compression),
                instance_name: self.config.cache.instance_name.clone(),
                digests: blob_digests,
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
        {
            Ok(response) => response,
            Err(status) => {
                return if matches!(status.code(), Code::InvalidArgument) {
                    warn!(
                        hash = &digest.hash,
                        "Attempted to download more blobs than the allowed limit"
                    );

                    Ok(vec![])
                } else {
                    Err(self.map_status_error("batch_read_blobs", status).into())
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
                        blob_hash = download.digest.as_ref().map(|d| &d.hash),
                        details = ?status.details,
                        code = ?code,
                        "Failed to download blob: {}",
                        color::muted_light(if status.message.is_empty() {
                            code.to_string()
                        } else {
                            status.message
                        }),
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
        let compression = self.config.cache.compression;
        let mut requests = vec![];

        trace!(
            hash = &digest.hash,
            compression = compression.to_string(),
            "Uploading {} output blobs",
            blobs.len()
        );

        for blob in blobs {
            requests.push(batch_update_blobs_request::Request {
                digest: Some(blob.digest),
                data: compress_blob(compression, blob.bytes)?,
                compressor: get_compressor(compression),
            });
        }

        let response = match self
            .get_cas_client()
            .batch_update_blobs(BatchUpdateBlobsRequest {
                instance_name: self.config.cache.instance_name.clone(),
                requests,
                digest_function: digest_function::Value::Sha256 as i32,
            })
            .await
        {
            Ok(response) => response,
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
                        hash = &digest.hash,
                        code = ?code,
                        "Remote service exhausted resource: {}",
                        color::muted_light(status.message()),
                    );

                    Ok(vec![])
                } else {
                    Err(self.map_status_error("batch_update_blobs", status).into())
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
                        blob_hash = upload.digest.as_ref().map(|d| &d.hash),
                        details = ?status.details,
                        code = ?code,
                        "Failed to upload blob: {}",
                        color::muted_light(if status.message.is_empty() {
                            code.to_string()
                        } else {
                            status.message
                        }),
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

    async fn stream_update_blob(
        &self,
        digest: &Digest,
        blob: Blob,
    ) -> miette::Result<Option<Digest>> {
        trace!(
            hash = &digest.hash,
            blob_hash = &blob.digest.hash,
            "Streaming upload output blob"
        );

        let resource_name = self.get_resource_name(&blob.digest);
        let total_bytes = blob.digest.size_bytes;
        let stream_error = Arc::new(Mutex::new(None));
        let stream_error_clone = stream_error.clone();

        let stream = async_stream::stream! {
            let reader = ReaderStream::new(blob.bytes.as_slice());
            let mut written_bytes: i64 = 0;

            for await read_result in reader {
                match read_result {
                    Ok(chunk) => {
                        let write_offset = written_bytes;
                        written_bytes += chunk.len() as i64;

                        yield WriteRequest {
                            resource_name: resource_name.clone(),
                            write_offset,
                            finish_write: written_bytes >= total_bytes,
                            data: chunk.to_vec(),
                        }
                    },
                    Err(error) => {
                        *stream_error_clone.lock().await = Some(error);
                    },
                }
            }
        };

        let result = self.get_bs_client().write(Request::new(stream)).await;

        if let Some(ref error) = *stream_error.lock().await {
            warn!(
                hash = &digest.hash,
                blob_hash = &blob.digest.hash,
                "Failed to stream upload blob: {}",
                color::muted_light(error.to_string()),
            );

            return Ok(None);
        }

        match result {
            Ok(response) => {
                let result = response.into_inner();

                if result.committed_size != -1 && result.committed_size < total_bytes {
                    warn!(
                        hash = &digest.hash,
                        blob_hash = &blob.digest.hash,
                        "Failed to upload blob, streamed bytes was {}, but we required {total_bytes}",
                        result.committed_size
                    );

                    return Ok(None);
                }
            }
            Err(status) => {
                return Err(self.map_status_error("stream_update_blob", status).into());
            }
        };

        Ok(Some(blob.digest))
    }
}
