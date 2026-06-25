use crate::compressable_blob::*;
use crate::grpc_services::*;
use crate::grpc_tls::*;
use crate::headers::extract_headers;
use crate::remote_error::RemoteError;
use async_trait::async_trait;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    BatchReadBlobsRequest, BatchUpdateBlobsRequest, FindMissingBlobsRequest,
    GetActionResultRequest, GetCapabilitiesRequest, ServerCapabilities, UpdateActionResultRequest,
    action_cache_client::ActionCacheClient, batch_update_blobs_request,
    capabilities_client::CapabilitiesClient,
    content_addressable_storage_client::ContentAddressableStorageClient,
};
use bazel_remote_apis::google::bytestream::{
    ReadRequest, WriteRequest, byte_stream_client::ByteStreamClient,
};
use moon_blob::{Blob, BlobContent, BlobSource};
use moon_cache_storage::{
    CacheCapabilities, CacheContext, Compressor, DigestFunction, InternalDigestExt, Manifest,
    StorageBackend,
};
use moon_common::{Id, color, is_ci, is_remote};
use moon_config::RemoteCompression;
use moon_hash::Digest;
use reqwest::header::HeaderMap;
use rustc_hash::FxHashSet;
use starbase_utils::fs;
use std::fmt::Debug;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::io::ReaderStream;
use tonic::{
    Code, Request,
    codegen::tokio_stream::StreamExt,
    transport::{Channel, Endpoint},
};
use tower::{ServiceBuilder, limit::ConcurrencyLimit, timeout::Timeout};
use tracing::{debug, error, trace, warn};

type LayeredService = Timeout<ConcurrencyLimit<RequestHeaders<Channel>>>;

pub struct GrpcRemoteStorage {
    context: CacheContext,
    id: Id,

    // States
    cache_enabled: OnceLock<bool>,
    capabilities: OnceLock<CacheCapabilities>,
    channel: OnceLock<Channel>,
    uuid: OnceLock<uuid::Uuid>,

    // Clients
    ac_client: OnceLock<ActionCacheClient<LayeredService>>,
    bs_client: OnceLock<ByteStreamClient<LayeredService>>,
    cap_client: OnceLock<CapabilitiesClient<LayeredService>>,
    cas_client: OnceLock<ContentAddressableStorageClient<LayeredService>>,
}

impl Debug for GrpcRemoteStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcRemoteStorage")
            .field("id", &self.id)
            .field("capabilities", &self.capabilities)
            .field("context", &self.context)
            .finish()
    }
}

impl GrpcRemoteStorage {
    pub fn new(context: CacheContext) -> miette::Result<Self> {
        Ok(Self {
            capabilities: OnceLock::new(),
            cache_enabled: OnceLock::new(),
            id: Id::raw("grpc-remote-cache"),
            context,
            channel: OnceLock::new(),
            ac_client: OnceLock::new(),
            bs_client: OnceLock::new(),
            cap_client: OnceLock::new(),
            cas_client: OnceLock::new(),
            uuid: OnceLock::new(),
        })
    }

    fn create_clients(&self, headers: HeaderMap) {
        let service: LayeredService = ServiceBuilder::new()
            .timeout(Duration::from_secs(60 * 60))
            .concurrency_limit(150)
            .layer(RequestHeadersLayer::new(headers))
            .service(self.channel.get().unwrap().clone());

        // Raise the max decoding message size from tonic's default of 4MB.
        // We already partition batches by size ourselves using the server's
        // MaxBatchTotalSizeBytes, but the server's gRPC frame limit and the
        // CAS batch size limit are separate concerns. The response also includes
        // protobuf overhead (digests, status, etc.) per blob that can push the
        // encoded response beyond 4MB even when blob data fits within the limit.
        let max_decode_size = 64 * 1024 * 1024; // 64MB

        let _ = self.ac_client.set(
            ActionCacheClient::new(service.clone()).max_decoding_message_size(max_decode_size),
        );

        let _ = self
            .bs_client
            .set(ByteStreamClient::new(service.clone()).max_decoding_message_size(max_decode_size));

        let _ = self.cap_client.set(
            CapabilitiesClient::new(service.clone()).max_decoding_message_size(max_decode_size),
        );

        let _ = self.cas_client.set(
            ContentAddressableStorageClient::new(service)
                .max_decoding_message_size(max_decode_size),
        );
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

    fn get_instance_name(&self) -> String {
        self.context.remote_config.cache.instance_name.clone()
    }

    fn get_uuid(&self) -> &uuid::Uuid {
        self.uuid.get_or_init(uuid::Uuid::new_v4)
    }

    fn validate_capabilities(&self, server_cap: &mut ServerCapabilities) {
        let storage = self.get_id().as_str();
        let host = &self.context.remote_config.host;
        let mut enabled = true;

        if let Some(cap) = &mut server_cap.cache_capabilities {
            let sha256_fn = DigestFunction::Sha256 as i32;

            if !cap.digest_functions.contains(&sha256_fn) {
                enabled = false;

                warn!(
                    storage,
                    host,
                    "Remote storage does not support SHA256 digests, which is required by moon, disabling backend"
                );
            }

            let compression = self.context.remote_config.cache.compression;
            let compressor = get_compressor(compression);

            if compression != RemoteCompression::None
                && (!cap.supported_compressors.contains(&compressor)
                    || !cap.supported_batch_update_compressors.contains(&compressor))
            {
                cap.supported_compressors = vec![Compressor::Identity as i32];
                cap.supported_batch_update_compressors = vec![Compressor::Identity as i32];

                warn!(
                    storage,
                    host,
                    "Remote storage does not support {} compression, but it has been configured and enabled through the {} setting, falling back to no compression",
                    compression,
                    color::property("remote.cache.compression"),
                );
            }
        } else {
            enabled = false;

            warn!(
                storage,
                host, "Remote storage does not support caching, disabling backend"
            );
        }

        self.cache_enabled.set(enabled).ok();
    }

    fn map_status_error(&self, method: &str, error: tonic::Status) -> RemoteError {
        if self.context.remote_debug {
            error!("{method}: {:#?}", error);
        }

        RemoteError::GrpcCallFailed {
            error: Box::new(error),
        }
    }

    fn map_transport_error(&self, method: &str, error: tonic::transport::Error) -> RemoteError {
        if self.context.remote_debug {
            error!("{method}: {:#?}", error);
        }

        RemoteError::GrpcConnectFailed {
            error: Box::new(error),
        }
    }

    fn can_download(&self) -> bool {
        self.cache_enabled.get().cloned().unwrap_or_default()
    }

    fn can_upload(&self) -> bool {
        self.cache_enabled.get().cloned().unwrap_or_default()
            && (is_ci() || !self.context.remote_config.cache.local_read_only)
    }
}

#[async_trait]
impl StorageBackend for GrpcRemoteStorage {
    fn get_capabilities(&self) -> &CacheCapabilities {
        self.capabilities
            .get_or_init(|| CacheCapabilities::default())
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    fn is_enabled(&self) -> bool {
        self.context.remote_config.is_enabled()
            && self.channel.get().is_some()
            && self.cache_enabled.get().cloned().unwrap_or_default()
    }

    async fn connect(&self) -> miette::Result<()> {
        let config = &self.context.remote_config;

        if is_remote() && config.is_localhost() {
            warn!(
                storage = self.get_id().as_str(),
                host = &config.host,
                "Remote service is configured with a localhost endpoint, but we are in a CI environment; disabling service",
            );

            return Ok(());
        }

        debug!(
            storage = self.get_id().as_str(),
            instance = &config.cache.instance_name,
            "Connecting to gRPC host {} {}",
            color::url(config.get_host()),
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

        // Extract headers and abort early if not enabled
        let Some(headers) = extract_headers(config)? else {
            return Ok(());
        };

        // Although we use a grpc(s) protocol for the host,
        // tonic only supports http(s), so change it
        let url = if let Some(suffix) = config.get_host().strip_prefix("grpc") {
            format!("http{suffix}")
        } else {
            config.get_host().to_owned()
        };

        let mut endpoint = Endpoint::from_shared(url)
            .map_err(|error| self.map_transport_error("host", error))?
            .user_agent("moon")
            .map_err(|error| self.map_transport_error("user_agent", error))?
            .keep_alive_while_idle(true)
            .tcp_keepalive(Some(Duration::from_secs(60)));

        if let Some(mtls) = &config.mtls {
            endpoint = endpoint
                .tls_config(create_mtls_config(mtls, &self.context.workspace_root)?)
                .map_err(|error| self.map_transport_error("tls", error))?;
        } else if let Some(tls) = &config.tls {
            endpoint = endpoint
                .tls_config(create_tls_config(tls, &self.context.workspace_root)?)
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
        let _ = if config.is_bearer_auth() {
            self.channel.set(endpoint.connect_lazy())
        } else {
            self.channel.set(
                endpoint
                    .connect()
                    .await
                    .map_err(|error| self.map_transport_error("connect_to_host", error))?,
            )
        };

        self.create_clients(headers);

        // Load and validate the capabilities
        let mut server_cap = self
            .get_cap_client()
            .get_capabilities(GetCapabilitiesRequest {
                instance_name: config.cache.instance_name.clone(),
            })
            .await
            .map_err(|error| self.map_status_error("load_capabilities", error))?
            .into_inner();

        self.validate_capabilities(&mut server_cap);

        if let Some(cache_cap) = server_cap.cache_capabilities {
            self.capabilities
                .set(CacheCapabilities::from_bazel_capabilities(cache_cap))
                .ok();
        }

        Ok(())
    }

    async fn retrieve_manifest(&self, digest: Digest) -> miette::Result<Option<Manifest>> {
        if !self.can_download() {
            return Ok(None);
        }

        match self
            .get_ac_client()
            .get_action_result(GetActionResultRequest {
                instance_name: self.get_instance_name(),
                action_digest: Some(digest.to_external_digest()),
                inline_stderr: true,
                inline_stdout: true,
                digest_function: DigestFunction::Sha256 as i32,
                ..Default::default()
            })
            .await
        {
            Ok(response) => {
                let result = response.into_inner();

                Ok(Some(Manifest::from_bazel_action_result(result)?))
            }
            Err(status) => {
                if matches!(status.code(), Code::NotFound) {
                    Ok(None)
                }
                // If we hit an out of range error, the payload is larger than the gRPC
                // limit, and will fail the entire pipeline. Instead of letting that
                // happen, let's just do a cache miss instead...
                else if matches!(status.code(), Code::OutOfRange) {
                    trace!(
                        hash = digest.hash.as_str(),
                        "Cache miss because the expected payload is too large"
                    );

                    Ok(None)
                } else {
                    Err(self.map_status_error("retrieve_manifest", status).into())
                }
            }
        }
    }

    async fn store_manifest(&self, digest: Digest, manifest: Manifest) -> miette::Result<()> {
        if !self.can_upload() {
            return Ok(());
        }

        match self
            .get_ac_client()
            .update_action_result(UpdateActionResultRequest {
                instance_name: self.get_instance_name(),
                action_digest: Some(digest.to_external_digest()),
                action_result: Some(manifest.into_bazel_action_result()),
                digest_function: DigestFunction::Sha256 as i32,
                ..Default::default()
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(status) => {
                if matches!(status.code(), Code::ResourceExhausted) {
                    Err(RemoteError::GrpcOutOfStorageSpace {
                        error: Box::new(status),
                    }
                    .into())
                } else {
                    Err(self.map_status_error("store_manifest", status).into())
                }
            }
        }
    }

    async fn find_missing_blobs(
        &self,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>> {
        match self
            .get_cas_client()
            .find_missing_blobs(FindMissingBlobsRequest {
                instance_name: self.get_instance_name(),
                blob_digests: blob_digests
                    .into_iter()
                    .map(|digest| digest.to_external_digest())
                    .collect(),
                digest_function: DigestFunction::Sha256 as i32,
            })
            .await
        {
            Ok(response) => {
                let mut digests = FxHashSet::default();

                for digest in response.into_inner().missing_blob_digests {
                    digests.insert(Digest::from_external(digest)?);
                }

                Ok(digests)
            }
            Err(status) => Err(self.map_status_error("find_missing_blobs", status).into()),
        }
    }

    async fn retrieve_blobs(
        &self,
        mut blob_digests: Vec<Digest>,
        stream: bool,
    ) -> miette::Result<Vec<Blob>> {
        if stream && blob_digests.len() == 1 {
            return self
                .retrieve_blob_streamed(blob_digests.remove(0))
                .await
                .map(|blob| blob.map(|blob| vec![blob.inner]).unwrap_or_default());
        }

        let response = match self
            .get_cas_client()
            .batch_read_blobs(BatchReadBlobsRequest {
                acceptable_compressors: get_acceptable_compressors(
                    self.context.remote_config.cache.compression,
                ),
                instance_name: self.get_instance_name(),
                digests: blob_digests
                    .into_iter()
                    .map(|digest| digest.to_external_digest())
                    .collect(),
                digest_function: DigestFunction::Sha256 as i32,
            })
            .await
        {
            Ok(response) => response,
            Err(status) => return Err(self.map_status_error("retrieve_blobs", status).into()),
        };

        let mut blobs = vec![];

        for download in response.into_inner().responses {
            let mut success = true;

            if let Some(status) = download.status {
                let code = Code::from_i32(status.code);

                if !matches!(code, Code::Ok | Code::NotFound) {
                    warn!(
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

                    success = false;
                }
            }

            if success && let Some(digest) = download.digest {
                let mut blob = CompressableBlob::new(Digest::from_external(digest)?, download.data);
                blob.compression = get_compression_from_code(download.compressor);
                blob.decompress()?;

                // Verify digest matches decompressed content
                let actual_digest = Digest::from_bytes(&blob.bytes)?;

                if actual_digest != blob.digest {
                    return Err(RemoteError::GrpcDownloadDigestMismatch {
                        actual: actual_digest,
                        expected: blob.digest.clone(),
                    }
                    .into());
                }

                blobs.push(blob.inner);
            }
        }

        Ok(blobs)
    }

    async fn store_blobs(
        &self,
        blob_sources: Vec<BlobSource>,
        stream: bool,
    ) -> miette::Result<u16> {
        let compression = self.context.remote_config.cache.compression;
        let mut blobs = vec![];

        for source in blob_sources {
            let bytes = match source.content {
                BlobContent::Inline(bytes) => Vec::from(bytes),
                BlobContent::File(rel_path) => {
                    fs::read_file_bytes(rel_path.to_logical_path(&self.context.workspace_root))?
                }
            };

            let mut blob = CompressableBlob::new(source.digest, bytes);
            blob.compress(compression)?;

            blobs.push(blob);
        }

        if stream && blobs.len() == 1 {
            return self.store_blob_streamed(blobs.remove(0)).await.map(|_| 1);
        }

        let response = match self
            .get_cas_client()
            .batch_update_blobs(BatchUpdateBlobsRequest {
                instance_name: self.get_instance_name(),
                requests: blobs
                    .into_iter()
                    .map(|blob| batch_update_blobs_request::Request {
                        data: blob.inner.bytes.to_vec(),
                        digest: Some(blob.inner.digest.into_external_digest()),
                        compressor: get_compressor(compression),
                    })
                    .collect(),
                digest_function: DigestFunction::Sha256 as i32,
            })
            .await
        {
            Ok(response) => response,
            Err(status) => {
                return if matches!(status.code(), Code::ResourceExhausted) {
                    Err(RemoteError::GrpcOutOfStorageSpace {
                        error: Box::new(status),
                    }
                    .into())
                } else {
                    Err(self.map_status_error("store_blobs", status).into())
                };
            }
        };

        let mut uploaded_count = 0;

        for upload in response.into_inner().responses {
            let mut success = true;

            if let Some(status) = upload.status {
                let code = Code::from_i32(status.code);

                if !matches!(code, Code::Ok) {
                    warn!(
                        blob_hash = upload.digest.as_ref().map(|dig| &dig.hash),
                        details = ?status.details,
                        code = ?code,
                        "Failed to upload blob: {}",
                        color::muted_light(if status.message.is_empty() {
                            code.to_string()
                        } else {
                            status.message
                        }),
                    );

                    success = false;
                }
            }

            if success && upload.digest.is_some() {
                uploaded_count += 1;
            }
        }

        Ok(uploaded_count)
    }
}

// STREAMING BLOB SUPPORT

impl GrpcRemoteStorage {
    async fn retrieve_blob_streamed(
        &self,
        blob_digest: Digest,
    ) -> miette::Result<Option<CompressableBlob>> {
        let resource_name = format!(
            "{}/blobs/{}/{}",
            self.get_instance_name(),
            blob_digest.hash,
            blob_digest.size,
        );

        let response = match self
            .get_bs_client()
            .read(ReadRequest {
                resource_name,
                read_offset: 0,
                read_limit: 0,
            })
            .await
        {
            Ok(response) => response,
            Err(status) => {
                return if matches!(status.code(), Code::NotFound) {
                    Ok(None)
                } else {
                    Err(self
                        .map_status_error("retrieve_blob_streamed", status)
                        .into())
                };
            }
        };

        let mut stream = response.into_inner();
        let mut bytes = Vec::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(data) => {
                    bytes.extend(data.data);
                }
                Err(error) => {
                    return Err(RemoteError::GrpcStreamDownloadFailed {
                        error: Box::new(error),
                    }
                    .into());
                }
            }
        }

        let blob = CompressableBlob::from_bytes(bytes)?;

        if blob.digest != blob_digest {
            return Err(RemoteError::GrpcDownloadDigestMismatch {
                actual: blob.digest.clone(),
                expected: blob_digest.clone(),
            }
            .into());
        }

        Ok(Some(blob))
    }

    async fn store_blob_streamed(&self, blob: CompressableBlob) -> miette::Result<Digest> {
        let resource_name = format!(
            "{}/uploads/{}/blobs/{}/{}",
            self.get_instance_name(),
            self.get_uuid(),
            blob.digest.hash,
            blob.digest.size,
        );
        let total_bytes = blob.digest.size;
        let stream_error = Arc::new(Mutex::new(None));
        let stream_error_clone = stream_error.clone();

        let stream = async_stream::stream! {
            let reader = ReaderStream::new(blob.inner.bytes.as_ref());
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
                        break;
                    },
                }
            }
        };

        let result = self.get_bs_client().write(Request::new(stream)).await;

        if let Some(error) = Arc::into_inner(stream_error).and_then(|error| error.into_inner()) {
            return Err(RemoteError::GrpcStreamUploadFailed {
                error: Box::new(error),
            }
            .into());
        }

        match result {
            Ok(response) => {
                let result = response.into_inner();

                if result.committed_size != total_bytes {
                    return Err(RemoteError::GrpcUploadBytesMismatch {
                        actual: result.committed_size,
                        expected: total_bytes,
                    }
                    .into());
                }
            }
            Err(status) => {
                return Err(self.map_status_error("store_blob_streamed", status).into());
            }
        };

        Ok(blob.inner.digest)
    }
}
