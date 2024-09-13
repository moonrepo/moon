use schematic::{derive_enum, validate, Config, ConfigEnum};
use std::path::PathBuf;

derive_enum!(
    #[derive(Copy, ConfigEnum)]
    pub enum RemoteCacheCompression {
        Gzip,
    }
);

/// Configures the caching service (powered by Bazel Remote Asset API).
#[derive(Clone, Config, Debug)]
pub struct RemoteTlsConfig {
    pub domain_name: String,
    pub pem_file: PathBuf,
}

#[derive(Clone, Config, Debug)]
pub struct RemoteCacheConfig {
    /// Compress artifacts when uploading and downloading. When enabled,
    /// the server must also be configured for compression.
    pub compression: Option<RemoteCacheCompression>,
}

#[derive(Clone, Config, Debug)]
pub struct RemoteConfig {
    #[setting(nested)]
    pub cache: RemoteCacheConfig,

    #[setting(validate = validate::not_empty)]
    pub host: String,

    #[setting(nested)]
    pub tls: Option<RemoteTlsConfig>,
}
