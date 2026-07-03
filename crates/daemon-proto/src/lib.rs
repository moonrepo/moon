#![allow(clippy::disallowed_types)]

tonic::include_proto!("moon.daemon.v1");

/// Version of the daemon RPC contract. Bump this whenever a change to
/// `daemon.proto` or a handler's semantics would make an already-running
/// daemon incompatible with a newer client, so the client restarts it
/// instead of talking to a daemon it can't fully understand.
pub const PROTOCOL_VERSION: u32 = 1;
