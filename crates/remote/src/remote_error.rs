use bazel_remote_apis::build::bazel::remote::execution::v2::Digest;
use miette::Diagnostic;
use moon_config::RemoteCompression;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum RemoteError {
    #[diagnostic(code(remote::grpc::call_failed))]
    #[error("Failed to make gRPC call.\n{}: {}", .error.code(), .error.message())]
    GrpcCallFailed { error: Box<tonic::Status> },

    #[diagnostic(code(remote::grpc::connect_failed))]
    #[error("Failed to connect to gRPC host.")]
    GrpcConnectFailed {
        #[source]
        error: Box<tonic::transport::Error>,
    },

    #[diagnostic(code(remote::grpc::out_of_storage_space))]
    #[error("Remote service is out of storage space.")]
    GrpcOutOfStorageSpace {
        #[source]
        error: Box<tonic::Status>,
    },

    #[diagnostic(code(remote::grpc::download_digest_mismatch))]
    #[error(
        "Failed to download blob, mismatched blob digests. Received {}:{}, but we expected {}:{}.",
        .actual.hash,
        .actual.size_bytes,
        .expected.hash,
        .expected.size_bytes,
    )]
    GrpcDownloadDigestMismatch { actual: Digest, expected: Digest },

    #[diagnostic(code(remote::grpc::upload_bytes_mismatch))]
    #[error("Failed to upload blob. Received bytes was {actual}, but we expected {expected}.")]
    GrpcUploadBytesMismatch { actual: i64, expected: i64 },

    #[diagnostic(code(remote::grpc::stream_download_failed))]
    #[error("Failed to stream download blob.")]
    GrpcStreamDownloadFailed {
        #[source]
        error: Box<tonic::Status>,
    },

    #[diagnostic(code(remote::grpc::stream_upload_failed))]
    #[error("Failed to stream upload blob.")]
    GrpcStreamUploadFailed {
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(remote::http::call_failed))]
    #[error("Failed to make HTTP call.")]
    HttpCallFailed {
        #[source]
        error: Box<reqwest::Error>,
    },

    #[diagnostic(code(remote::http::connect_failed))]
    #[error("Failed to connect to HTTP host ({code} {reason}).")]
    HttpConnectFailed { code: u16, reason: String },

    #[diagnostic(code(remote::http::request_failed))]
    #[error("Failed to make HTTP request ({status}).")]
    HttpRequestFailed { status: Box<reqwest::StatusCode> },

    #[diagnostic(code(remote::compression_failed))]
    #[error("Failed to compress blob using {format}.")]
    CompressFailed {
        format: RemoteCompression,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(remote::compression_failed))]
    #[error("Failed to decompress blob using {format}.")]
    DecompressFailed {
        format: RemoteCompression,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(remote::http::no_support))]
    #[error("The HTTP based remote service is currently not supported, use gRPC instead.")]
    NoHttpClient,

    #[diagnostic(code(remote::unsupported_protocol))]
    #[error("Unknown remote host protocol, only gRPC is supported.")]
    UnknownHostProtocol,
}
