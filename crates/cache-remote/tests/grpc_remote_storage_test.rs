use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionCacheUpdateCapabilities, ActionResult, BatchReadBlobsRequest, BatchReadBlobsResponse,
    BatchUpdateBlobsRequest, BatchUpdateBlobsResponse, CacheCapabilities as BazelCacheCapabilities,
    FindMissingBlobsRequest, FindMissingBlobsResponse, GetActionResultRequest,
    GetCapabilitiesRequest, GetTreeRequest, GetTreeResponse, ServerCapabilities, SpliceBlobRequest,
    SpliceBlobResponse, SplitBlobRequest, SplitBlobResponse, UpdateActionResultRequest,
    action_cache_server::{ActionCache, ActionCacheServer},
    batch_read_blobs_response, batch_update_blobs_response,
    capabilities_server::{Capabilities, CapabilitiesServer},
    compressor::Value as Compressor,
    content_addressable_storage_server::{
        ContentAddressableStorage, ContentAddressableStorageServer,
    },
    digest_function::Value as DigestFunction,
};
use bazel_remote_apis::google::bytestream::{
    QueryWriteStatusRequest, QueryWriteStatusResponse, ReadRequest, ReadResponse, WriteRequest,
    WriteResponse,
    byte_stream_server::{ByteStream, ByteStreamServer},
};
use moon_blob::{BlobContent, BlobInput, Bytes};
use moon_cache_remote::{GrpcRemoteStorage, RemoteError};
use moon_cache_storage::{CacheContext, Manifest, StorageBackend};
use moon_config::{CacheConfig, RemoteCompression, RemoteConfig};
use moon_hash::Digest;
use rustc_hash::FxHashMap;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tonic::codegen::tokio_stream::{self, Stream};
use tonic::transport::Server;
use tonic::transport::server::TcpIncoming;
use tonic::{Code, Request, Response, Status, Streaming};

// ---- In-memory mock of the Bazel Remote Execution API ----

/// Blob hash -> (raw data, compressor code).
type BlobStore = Arc<Mutex<FxHashMap<String, (Vec<u8>, i32)>>>;
/// Action digest hash -> result.
type ActionResultStore = Arc<Mutex<FxHashMap<String, ActionResult>>>;

/// Backs the four RE API services with shared in-memory maps. Behavior can be
/// tweaked per test (error injection, corrupt reads) via the builder methods.
#[derive(Clone, Default)]
struct MockBackend {
    blobs: BlobStore,
    action_results: ActionResultStore,

    get_action_status: Option<Code>,
    update_status: Option<Code>,
    corrupt_reads: bool,
    zstd_streaming: bool,
    zstd_batch: bool,
}

impl MockBackend {
    fn failing_get_action(mut self, code: Code) -> Self {
        self.get_action_status = Some(code);
        self
    }

    fn failing_update(mut self, code: Code) -> Self {
        self.update_status = Some(code);
        self
    }

    fn corrupting_reads(mut self) -> Self {
        self.corrupt_reads = true;
        self
    }

    fn with_zstd(mut self) -> Self {
        self.zstd_streaming = true;
        self.zstd_batch = true;
        self
    }

    /// Advertise zstd for the bytestream (streaming) path only, not batch.
    fn with_streaming_zstd(mut self) -> Self {
        self.zstd_streaming = true;
        self
    }
}

#[tonic::async_trait]
impl Capabilities for MockBackend {
    async fn get_capabilities(
        &self,
        _request: Request<GetCapabilitiesRequest>,
    ) -> Result<Response<ServerCapabilities>, Status> {
        let with_zstd = vec![Compressor::Identity as i32, Compressor::Zstd as i32];
        let identity_only = vec![Compressor::Identity as i32];

        Ok(Response::new(ServerCapabilities {
            cache_capabilities: Some(BazelCacheCapabilities {
                digest_functions: vec![DigestFunction::Sha256 as i32],
                supported_compressors: if self.zstd_streaming {
                    with_zstd.clone()
                } else {
                    identity_only.clone()
                },
                supported_batch_update_compressors: if self.zstd_batch {
                    with_zstd
                } else {
                    identity_only
                },
                max_batch_total_size_bytes: 4 * 1024 * 1024,
                action_cache_update_capabilities: Some(ActionCacheUpdateCapabilities {
                    update_enabled: true,
                }),
                ..Default::default()
            }),
            ..Default::default()
        }))
    }
}

#[tonic::async_trait]
impl ActionCache for MockBackend {
    async fn get_action_result(
        &self,
        request: Request<GetActionResultRequest>,
    ) -> Result<Response<ActionResult>, Status> {
        if let Some(code) = self.get_action_status {
            return Err(Status::new(code, "injected"));
        }

        let hash = request
            .into_inner()
            .action_digest
            .map(|digest| digest.hash)
            .unwrap_or_default();

        match self.action_results.lock().unwrap().get(&hash) {
            Some(result) => Ok(Response::new(result.clone())),
            None => Err(Status::not_found("missing")),
        }
    }

    async fn update_action_result(
        &self,
        request: Request<UpdateActionResultRequest>,
    ) -> Result<Response<ActionResult>, Status> {
        if let Some(code) = self.update_status {
            return Err(Status::new(code, "injected"));
        }

        let request = request.into_inner();
        let hash = request
            .action_digest
            .map(|digest| digest.hash)
            .unwrap_or_default();
        let result = request.action_result.unwrap_or_default();

        self.action_results
            .lock()
            .unwrap()
            .insert(hash, result.clone());

        Ok(Response::new(result))
    }
}

#[tonic::async_trait]
impl ContentAddressableStorage for MockBackend {
    type GetTreeStream = Pin<Box<dyn Stream<Item = Result<GetTreeResponse, Status>> + Send>>;

    async fn find_missing_blobs(
        &self,
        request: Request<FindMissingBlobsRequest>,
    ) -> Result<Response<FindMissingBlobsResponse>, Status> {
        let blobs = self.blobs.lock().unwrap();
        let missing = request
            .into_inner()
            .blob_digests
            .into_iter()
            .filter(|digest| !blobs.contains_key(&digest.hash))
            .collect();

        Ok(Response::new(FindMissingBlobsResponse {
            missing_blob_digests: missing,
        }))
    }

    async fn batch_update_blobs(
        &self,
        request: Request<BatchUpdateBlobsRequest>,
    ) -> Result<Response<BatchUpdateBlobsResponse>, Status> {
        let request = request.into_inner();
        let mut blobs = self.blobs.lock().unwrap();
        let mut responses = vec![];

        for blob in request.requests {
            // Mimic a server that rejects a compressor it didn't advertise in
            // `supported_batch_update_compressors`.
            if blob.compressor == Compressor::Zstd as i32 && !self.zstd_batch {
                return Err(Status::invalid_argument("unsupported compressor"));
            }

            if let Some(digest) = blob.digest.clone() {
                blobs.insert(digest.hash.clone(), (blob.data, blob.compressor));
                responses.push(batch_update_blobs_response::Response {
                    digest: Some(digest),
                    status: None,
                });
            }
        }

        Ok(Response::new(BatchUpdateBlobsResponse { responses }))
    }

    async fn batch_read_blobs(
        &self,
        request: Request<BatchReadBlobsRequest>,
    ) -> Result<Response<BatchReadBlobsResponse>, Status> {
        let request = request.into_inner();
        let blobs = self.blobs.lock().unwrap();
        let mut responses = vec![];

        for digest in request.digests {
            if let Some((data, compressor)) = blobs.get(&digest.hash) {
                let data = if self.corrupt_reads {
                    b"tampered".to_vec()
                } else {
                    data.clone()
                };

                responses.push(batch_read_blobs_response::Response {
                    digest: Some(digest),
                    data,
                    compressor: *compressor,
                    status: None,
                });
            }
        }

        Ok(Response::new(BatchReadBlobsResponse { responses }))
    }

    async fn get_tree(
        &self,
        _request: Request<GetTreeRequest>,
    ) -> Result<Response<Self::GetTreeStream>, Status> {
        Err(Status::unimplemented(
            "get_tree is not supported by the mock",
        ))
    }

    async fn split_blob(
        &self,
        _request: Request<SplitBlobRequest>,
    ) -> Result<Response<SplitBlobResponse>, Status> {
        Err(Status::unimplemented(
            "split_blob is not supported by the mock",
        ))
    }

    async fn splice_blob(
        &self,
        _request: Request<SpliceBlobRequest>,
    ) -> Result<Response<SpliceBlobResponse>, Status> {
        Err(Status::unimplemented(
            "splice_blob is not supported by the mock",
        ))
    }
}

#[tonic::async_trait]
impl ByteStream for MockBackend {
    type ReadStream = Pin<Box<dyn Stream<Item = Result<ReadResponse, Status>> + Send>>;

    async fn read(
        &self,
        request: Request<ReadRequest>,
    ) -> Result<Response<Self::ReadStream>, Status> {
        let name = request.into_inner().resource_name;
        let data = hash_from_resource(&name).and_then(|hash| {
            self.blobs
                .lock()
                .unwrap()
                .get(&hash)
                .map(|(data, _)| data.clone())
        });

        match data {
            Some(data) => {
                // Chunk the response to exercise the client's accumulation loop.
                let chunks: Vec<Result<ReadResponse, Status>> = data
                    .chunks(4)
                    .map(|chunk| {
                        Ok(ReadResponse {
                            data: chunk.to_vec(),
                        })
                    })
                    .collect();

                Ok(Response::new(Box::pin(tokio_stream::iter(chunks))))
            }
            None => Err(Status::not_found("missing")),
        }
    }

    async fn write(
        &self,
        request: Request<Streaming<WriteRequest>>,
    ) -> Result<Response<WriteResponse>, Status> {
        let mut stream = request.into_inner();
        let mut data = Vec::new();
        let mut resource_name = String::new();

        while let Some(chunk) = stream.message().await? {
            if !chunk.resource_name.is_empty() {
                resource_name = chunk.resource_name;
            }

            data.extend(chunk.data);
        }

        let committed_size = data.len() as i64;

        if let Some(hash) = hash_from_resource(&resource_name) {
            self.blobs
                .lock()
                .unwrap()
                .insert(hash, (data, Compressor::Identity as i32));
        }

        Ok(Response::new(WriteResponse { committed_size }))
    }

    async fn query_write_status(
        &self,
        _request: Request<QueryWriteStatusRequest>,
    ) -> Result<Response<QueryWriteStatusResponse>, Status> {
        Err(Status::unimplemented(
            "query_write_status is not supported by the mock",
        ))
    }
}

/// Extracts the blob hash from a ByteStream resource name. Both the read form
/// (`{instance}/blobs/{hash}/{size}`) and the write form
/// (`{instance}/uploads/{uuid}/blobs/{hash}/{size}`) place the hash right after
/// the `blobs` segment.
fn hash_from_resource(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.split('/').collect();

    // Compressed form: `.../compressed-blobs/{compressor}/{hash}/{size}`.
    if let Some(index) = parts.iter().position(|part| *part == "compressed-blobs") {
        return parts.get(index + 2).map(|hash| hash.to_string());
    }

    // Uncompressed form: `.../blobs/{hash}/{size}`.
    let index = parts.iter().position(|part| *part == "blobs")?;
    parts.get(index + 1).map(|hash| hash.to_string())
}

// ---- Harness ----

async fn spawn_server(backend: MockBackend) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = TcpIncoming::from(listener);

    tokio::spawn(async move {
        Server::builder()
            .add_service(CapabilitiesServer::new(backend.clone()))
            .add_service(ActionCacheServer::new(backend.clone()))
            .add_service(ContentAddressableStorageServer::new(backend.clone()))
            .add_service(ByteStreamServer::new(backend))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    format!("grpc://{addr}")
}

fn create_storage(
    sandbox: &Sandbox,
    host: String,
    compression: RemoteCompression,
) -> GrpcRemoteStorage {
    let mut remote = RemoteConfig {
        host: Some(host),
        ..Default::default()
    };
    remote.cache.instance_name = "moon-test".to_owned();
    remote.cache.compression = compression;

    let context = CacheContext {
        cache_dir: sandbox.path().join(".moon/cache"),
        cache_config: Arc::new(CacheConfig::default()),
        config_dir: sandbox.path().join(".moon"),
        remote_config: Arc::new(remote),
        remote_debug: false,
        workspace_root: sandbox.path().to_path_buf(),
    };

    GrpcRemoteStorage::new(context).unwrap()
}

fn digest_of(bytes: &[u8]) -> Digest {
    Digest::from_bytes(bytes).unwrap()
}

async fn connect(backend: MockBackend) -> (Sandbox, GrpcRemoteStorage) {
    connect_with(backend, RemoteCompression::None).await
}

async fn connect_with(
    backend: MockBackend,
    compression: RemoteCompression,
) -> (Sandbox, GrpcRemoteStorage) {
    let host = spawn_server(backend).await;
    let sandbox = create_empty_sandbox();
    let storage = create_storage(&sandbox, host, compression);

    storage.connect().await.unwrap();

    (sandbox, storage)
}

fn remote_error(report: miette::Report) -> RemoteError {
    report.downcast::<RemoteError>().expect("a RemoteError")
}

mod grpc_remote_storage {
    use super::*;

    mod connect {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn negotiates_capabilities_and_enables() {
            let (_sandbox, storage) = connect(MockBackend::default()).await;

            assert!(storage.is_readable());
        }
    }

    mod manifests {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn store_and_retrieve_round_trip() {
            let (_sandbox, storage) = connect(MockBackend::default()).await;
            let digest = digest_of(b"action");

            storage
                .store_manifest(
                    digest.clone(),
                    Manifest {
                        exit_code: 9,
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let result = storage.retrieve_manifest(digest).await.unwrap();

            assert_eq!(result.unwrap().exit_code, 9);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn retrieve_unknown_returns_none() {
            let (_sandbox, storage) = connect(MockBackend::default()).await;

            let result = storage
                .retrieve_manifest(digest_of(b"missing"))
                .await
                .unwrap();

            assert!(result.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn retrieve_treats_out_of_range_as_cache_miss() {
            // A payload larger than the gRPC limit surfaces as OutOfRange; the
            // client downgrades it to a miss rather than failing the pipeline.
            let backend = MockBackend::default().failing_get_action(Code::OutOfRange);
            let (_sandbox, storage) = connect(backend).await;

            let result = storage.retrieve_manifest(digest_of(b"big")).await.unwrap();

            assert!(result.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn store_maps_resource_exhausted() {
            let backend = MockBackend::default().failing_update(Code::ResourceExhausted);
            let (_sandbox, storage) = connect(backend).await;

            let error = storage
                .store_manifest(digest_of(b"action"), Manifest::default())
                .await
                .unwrap_err();

            assert!(matches!(
                remote_error(error),
                RemoteError::GrpcOutOfStorageSpace { .. }
            ));
        }
    }

    mod blobs {
        use super::*;

        fn inline(content: &'static [u8]) -> BlobInput {
            BlobInput {
                content: BlobContent::Inline(Bytes::from_static(content)),
                digest: digest_of(content),
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn store_and_retrieve_round_trip() {
            let (_sandbox, storage) = connect(MockBackend::default()).await;
            let content = b"blob content";
            let digest = digest_of(content);

            let stored = storage
                .store_blobs(vec![inline(content)], false)
                .await
                .unwrap();
            assert_eq!(stored, vec![digest.clone()]);

            let blobs = storage.retrieve_blobs(vec![digest], false).await.unwrap();
            assert_eq!(blobs.len(), 1);
            assert_eq!(blobs[0].content.get_bytes().unwrap(), content);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn find_missing_returns_only_absent() {
            let (_sandbox, storage) = connect(MockBackend::default()).await;

            // Store one blob so it's present, then query alongside an absent one.
            storage
                .store_blobs(vec![inline(b"present")], false)
                .await
                .unwrap();

            let absent = digest_of(b"absent");
            let missing = storage
                .find_missing_blobs(vec![digest_of(b"present"), absent.clone()])
                .await
                .unwrap();

            assert_eq!(missing.len(), 1);
            assert!(missing.contains(&absent));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn retrieve_detects_digest_mismatch() {
            let (_sandbox, storage) = connect(MockBackend::default().corrupting_reads()).await;
            let content = b"original";
            let digest = digest_of(content);

            storage
                .store_blobs(vec![inline(content)], false)
                .await
                .unwrap();

            let error = storage
                .retrieve_blobs(vec![digest], false)
                .await
                .unwrap_err();

            assert!(matches!(
                remote_error(error),
                RemoteError::GrpcDownloadDigestMismatch { .. }
            ));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn streamed_store_and_retrieve_round_trip() {
            // A single blob with `stream = true` goes through the ByteStream
            // write/read APIs instead of the batch CAS APIs.
            let (_sandbox, storage) = connect(MockBackend::default()).await;
            let content = b"streamed blob content";
            let digest = digest_of(content);

            let stored = storage
                .store_blobs(vec![inline(content)], true)
                .await
                .unwrap();
            assert_eq!(stored, vec![digest.clone()]);

            let blobs = storage.retrieve_blobs(vec![digest], true).await.unwrap();
            assert_eq!(blobs.len(), 1);
            assert_eq!(blobs[0].content.get_bytes().unwrap(), content);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn streamed_retrieve_unknown_returns_empty() {
            let (_sandbox, storage) = connect(MockBackend::default()).await;

            let blobs = storage
                .retrieve_blobs(vec![digest_of(b"missing")], true)
                .await
                .unwrap();

            assert!(blobs.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn streamed_round_trip_with_zstd_compression() {
            // Streamed upload/download of a compressed blob uses the
            // `compressed-blobs/{compressor}/...` ByteStream resource form.
            let (_sandbox, storage) =
                connect_with(MockBackend::default().with_zstd(), RemoteCompression::Zstd).await;
            let content = b"streamed and compressed, streamed and compressed!";
            let digest = digest_of(content);

            let stored = storage
                .store_blobs(vec![inline(content)], true)
                .await
                .unwrap();
            assert_eq!(stored, vec![digest.clone()]);

            let blobs = storage.retrieve_blobs(vec![digest], true).await.unwrap();
            assert_eq!(blobs.len(), 1);
            assert_eq!(blobs[0].content.get_bytes().unwrap(), content);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn batch_round_trip_with_zstd_compression() {
            // The server advertises zstd and the client is configured for it, so
            // blobs are compressed on upload and decompressed on download.
            let (_sandbox, storage) =
                connect_with(MockBackend::default().with_zstd(), RemoteCompression::Zstd).await;
            let content = b"compress me, compress me, compress me!";
            let digest = digest_of(content);

            let stored = storage
                .store_blobs(vec![inline(content)], false)
                .await
                .unwrap();
            assert_eq!(stored, vec![digest.clone()]);

            let blobs = storage.retrieve_blobs(vec![digest], false).await.unwrap();
            assert_eq!(blobs.len(), 1);
            assert_eq!(blobs[0].content.get_bytes().unwrap(), content);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn falls_back_to_uncompressed_when_server_lacks_compression() {
            // The client is configured for zstd but the server advertises only
            // identity, so compression must be negotiated off — otherwise the
            // server rejects the unsupported compressor.
            let (_sandbox, storage) =
                connect_with(MockBackend::default(), RemoteCompression::Zstd).await;
            let content = b"would-be compressed content";
            let digest = digest_of(content);

            let stored = storage
                .store_blobs(vec![inline(content)], false)
                .await
                .unwrap();
            assert_eq!(stored, vec![digest.clone()]);

            let blobs = storage.retrieve_blobs(vec![digest], false).await.unwrap();
            assert_eq!(blobs.len(), 1);
            assert_eq!(blobs[0].content.get_bytes().unwrap(), content);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn compression_is_negotiated_per_path() {
            // Server supports zstd for the bytestream path but not batch-update,
            // so the two paths negotiate compression independently.
            let (_sandbox, storage) = connect_with(
                MockBackend::default().with_streaming_zstd(),
                RemoteCompression::Zstd,
            )
            .await;

            // Batched upload must fall back to identity (the mock rejects zstd
            // batches), while the streamed upload still uses zstd.
            let batched = b"batched payload";
            let batched_digest = digest_of(batched);
            storage
                .store_blobs(vec![inline(batched)], false)
                .await
                .unwrap();

            let streamed = b"streamed payload";
            let streamed_digest = digest_of(streamed);
            let stored = storage
                .store_blobs(vec![inline(streamed)], true)
                .await
                .unwrap();
            assert_eq!(stored, vec![streamed_digest.clone()]);

            // Both round-trip back to their original contents.
            let batched_back = storage
                .retrieve_blobs(vec![batched_digest], false)
                .await
                .unwrap();
            assert_eq!(batched_back[0].content.get_bytes().unwrap(), batched);

            let streamed_back = storage
                .retrieve_blobs(vec![streamed_digest], true)
                .await
                .unwrap();
            assert_eq!(streamed_back[0].content.get_bytes().unwrap(), streamed);
        }
    }
}
