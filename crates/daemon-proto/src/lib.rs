#![allow(clippy::disallowed_types)]

tonic::include_proto!("moon.daemon.v1");

/// Version of the daemon RPC contract. Bump this whenever a change to
/// `daemon.proto` or a handler's semantics would make an already-running
/// daemon incompatible with a newer client, so the client restarts it
/// instead of talking to a daemon it can't fully understand.
pub const PROTOCOL_VERSION: u32 = 2;

/// Maximum size of an encoded or decoded RPC message, for both the client
/// and the server. Raised from tonic's 4 MiB default, as task manifests carry
/// a task's stdout/stderr inline, and an entry for every output file, which
/// a verbose task or a large build easily exceeds. A larger message fails the
/// RPC, and the client falls back to doing the work in-process.
pub const MAX_MESSAGE_SIZE: usize = 256 * 1024 * 1024;
