mod blob;
mod digest_compat;
mod grpc_remote_client;
mod grpc_services;
mod grpc_tls;
mod helpers;
mod remote_client;
mod remote_error;
mod remote_service;

pub use bazel_remote_apis::build::bazel::remote::execution::v2::Digest as RemoteDigest;
pub use digest_compat::*;
pub use helpers::*;
pub use remote_error::*;
pub use remote_service::*;

// TODO:
// - Other digest functions besides sha256
// - Directory blob types
// - retries
// - metrics
