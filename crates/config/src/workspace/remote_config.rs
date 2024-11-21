use schematic::{validate, Config};
use std::path::PathBuf;

/// Configures the action cache (AC) and content addressable cache (CAS).
#[derive(Clone, Config, Debug)]
pub struct RemoteCacheConfig {
    #[setting(default = "moon-outputs")]
    pub instance_name: String,
}

#[derive(Clone, Config, Debug)]
pub struct RemoteTlsConfig {
    pub domain_name: String,
    pub pem_file: PathBuf,
}

/// Configures the remote service, powered by the Bazel Remote Execution API.
#[derive(Clone, Config, Debug)]
pub struct RemoteConfig {
    /// Configures the action cache (AC) and content addressable cache (CAS).
    #[setting(nested)]
    pub cache: RemoteCacheConfig,

    /// The remote host to connect and send requests to.
    /// Supports gRPC protocols.
    #[setting(validate = validate::not_empty)]
    pub host: String,

    #[setting(nested)]
    pub tls: Option<RemoteTlsConfig>,
}
