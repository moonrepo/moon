mod fs_digest;
mod grpc_remote_client;
mod remote_client;
mod remote_error;
mod remote_service;

pub use bazel_remote_apis::build::bazel::remote::execution::v2::Digest;
pub use fs_digest::*;
pub use remote_error::*;
pub use remote_service::*;

// TODO:
// - HTTP(S) client
// - Other digest functions besides sha256
// - Compression formats (only identity right now)
// - Proper error handling
// - Directory blob types
