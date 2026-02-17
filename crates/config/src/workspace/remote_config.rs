use crate::shapes::FilePath;
use crate::{config_struct, config_unit_enum, is_false};
use rustc_hash::FxHashMap;
use schematic::{Config, ConfigEnum, ValidateError, ValidateResult, env};

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

config_unit_enum!(
    /// The API format of the remote service.
    /// @since 1.32.0
    #[derive(ConfigEnum)]
    pub enum RemoteApi {
        /// gRPC(S) endpoints.
        #[default]
        Grpc,
        /// HTTP(S) endpoints.
        Http,
    }
);

config_struct!(
    /// Configures basic HTTP authentication.
    /// @since 1.32.0
    #[derive(Config)]
    pub struct RemoteAuthConfig {
        /// A map of HTTP headers to inject into every request.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub headers: FxHashMap<String, String>,

        /// The name of an environment variable to use as a bearer token.
        #[setting(env = "MOON_REMOTE_AUTH_TOKEN")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub token: Option<String>,
    }
);

config_unit_enum!(
    /// Supported blob compression levels for gRPC APIs.
    /// @since 1.31.0
    #[derive(ConfigEnum)]
    pub enum RemoteCompression {
        /// No compression.
        #[default]
        None,
        /// Zstandard compression.
        Zstd,
    }
);

impl RemoteCompression {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::None)
    }
}

config_struct!(
    /// Configures the action cache (AC) and content addressable cache (CAS).
    /// @since 1.30.0
    #[derive(Config)]
    #[config(env_prefix = "MOON_REMOTE_CACHE_")]
    pub struct RemoteCacheConfig {
        /// The compression format to use when uploading/downloading blobs.
        /// @since 1.31.0
        pub compression: RemoteCompression,

        /// Unique instance name for blobs. Will be used as a folder name.
        #[setting(default = "moon-outputs")]
        pub instance_name: String,

        /// When local, only download matching blobs and do not upload new
        /// blobs. Blobs will only be uploaded in CI environments.
        /// @since 1.40.0
        #[setting(parse_env = env::parse_bool)]
        #[serde(skip_serializing_if = "is_false")]
        pub local_read_only: bool,

        /// When downloading blobs, verify the digests/hashes in the response
        /// match the associated blob contents. This will reduce performance
        /// but ensure partial or corrupted blobs won't cause failures.
        /// @since 1.36.0
        #[setting(parse_env = env::parse_bool)]
        #[serde(skip_serializing_if = "is_false")]
        pub verify_integrity: bool,
    }
);

config_struct!(
    /// Configures for server-only authentication with TLS.
    /// @since 1.30.0
    #[derive(Config)]
    #[config(env_prefix = "MOON_REMOTE_TLS_")]
    pub struct RemoteTlsConfig {
        /// If true, assume that the server supports HTTP/2,
        /// even if it doesn't provide protocol negotiation via ALPN.
        #[setting(env = "MOON_REMOTE_TLS_HTTP2", parse_env = env::parse_bool)]
        #[serde(skip_serializing_if = "is_false")]
        pub assume_http2: bool,

        /// A file path, relative from the workspace root, to the
        /// certificate authority PEM encoded X509 certificate.
        #[setting(validate = path_is_required)]
        pub cert: FilePath,

        /// The domain name in which to verify the TLS certificate.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub domain: Option<String>,
    }
);

config_struct!(
    /// Configures for both server and client authentication with mTLS.
    /// @since 1.30.0
    #[derive(Config)]
    #[config(env_prefix = "MOON_REMOTE_MTLS_")]
    pub struct RemoteMtlsConfig {
        /// If true, assume that the server supports HTTP/2,
        /// even if it doesn't provide protocol negotiation via ALPN.
        #[setting(env = "MOON_REMOTE_MTLS_HTTP", parse_env = env::parse_bool)]
        #[serde(skip_serializing_if = "is_false")]
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
        #[serde(skip_serializing_if = "Option::is_none")]
        pub domain: Option<String>,
    }
);

config_struct!(
    /// Configures the remote service, powered by the Bazel Remote Execution API.
    /// @since 1.30.0
    #[derive(Config)]
    pub struct RemoteConfig {
        /// The API format of the remote service.
        /// @since 1.32.0
        #[setting(env = "MOON_REMOTE_API")]
        pub api: RemoteApi,

        /// Connect to the host using basic HTTP authentication.
        /// @since 1.32.0
        #[setting(nested)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub auth: Option<RemoteAuthConfig>,

        /// Configures the action cache (AC) and content addressable cache (CAS).
        #[setting(nested)]
        pub cache: RemoteCacheConfig,

        /// The remote host to connect and send requests to.
        /// Supports gRPC protocols.
        #[setting(env = "MOON_REMOTE_HOST")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub host: Option<String>,

        /// Connect to the host using server and client authentication with mTLS.
        /// This takes precedence over normal TLS.
        #[setting(nested)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub mtls: Option<RemoteMtlsConfig>,

        /// Connect to the host using server-only authentication with TLS.
        #[setting(nested)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tls: Option<RemoteTlsConfig>,
    }
);

impl RemoteConfig {
    pub fn get_host(&self) -> &str {
        self.host.as_deref().unwrap()
    }

    pub fn is_bearer_auth(&self) -> bool {
        self.auth.as_ref().is_some_and(|auth| auth.token.is_some())
    }

    pub fn is_enabled(&self) -> bool {
        self.host.as_ref().is_some_and(|host| !host.is_empty())
    }

    pub fn is_localhost(&self) -> bool {
        self.host
            .as_ref()
            .is_some_and(|host| host.contains("localhost") || host.contains("0.0.0.0"))
    }

    pub fn is_secure(&self) -> bool {
        self.is_bearer_auth() || self.tls.is_some() || self.mtls.is_some()
    }

    pub fn is_secure_protocol(&self) -> bool {
        self.host
            .as_ref()
            .is_some_and(|host| host.starts_with("https") || host.starts_with("grpcs"))
    }
}
