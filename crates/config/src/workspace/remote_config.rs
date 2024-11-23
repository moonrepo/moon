use crate::portable_path::FilePath;
use schematic::{validate, Config, ValidateError, ValidateResult};

fn path_is_required<D, C>(
    value: &FilePath,
    _data: &D,
    _context: &C,
    _finalize: bool,
) -> ValidateResult {
    if value.as_str().is_empty() {
        return Err(ValidateError::new("path must not be empty"));
    }

    Ok(())
}

/// Configures the action cache (AC) and content addressable cache (CAS).
#[derive(Clone, Config, Debug)]
pub struct RemoteCacheConfig {
    #[setting(default = "moon-outputs")]
    pub instance_name: String,
}

/// Configures for server-only authentication with TLS.
#[derive(Clone, Config, Debug)]
pub struct RemoteTlsConfig {
    /// If true, assume that the server supports HTTP/2,
    /// even if it doesn't provide protocol negotiation via ALPN.
    pub assume_http2: bool,

    /// A file path, relative from the workspace root, to the
    /// certificate authority PEM encoded X509 certificate.
    #[setting(validate = path_is_required)]
    pub cert: FilePath,

    /// The domain name in which to verify the TLS certificate.
    pub domain: Option<String>,
}

/// Configures for both server and client authentication with mTLS.
#[derive(Clone, Config, Debug)]
pub struct RemoteMtlsConfig {
    /// If true, assume that the server supports HTTP/2,
    /// even if it doesn't provide protocol negotiation via ALPN.
    pub assume_http2: bool,

    /// A file path, relative from the workspace root, to the
    /// certificate authority PEM encoded X509 certificate.
    #[setting(validate = path_is_required)]
    pub ca_cert: FilePath,

    /// A file path, relative from the workspace root, to the
    /// client's PEM encoded X509 certificate.
    #[setting(validate = path_is_required)]
    pub client_cert: FilePath,

    /// A file path, relative from the workspace root, to the
    /// client's PEM encoded X509 private key.
    #[setting(validate = path_is_required)]
    pub client_key: FilePath,

    /// The domain name in which to verify the TLS certificate.
    pub domain: Option<String>,
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

    /// Connect to the host using server and client authentication with mTLS.
    /// This takes precedence over normal TLS.
    #[setting(nested)]
    pub mtls: Option<RemoteMtlsConfig>,

    /// Connect to the host using server-only authentication with TLS.
    #[setting(nested)]
    pub tls: Option<RemoteTlsConfig>,
}

impl RemoteConfig {
    pub fn is_localhost(&self) -> bool {
        self.host.contains("localhost") || self.host.contains("0.0.0.0")
    }

    pub fn is_secure(&self) -> bool {
        self.tls.is_some() || self.mtls.is_some()
    }
}
