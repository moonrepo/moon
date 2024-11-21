use schematic::{validate, Config};
use std::path::PathBuf;

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

#[derive(Clone, Config, Debug)]
pub struct RemoteConfig {
    #[setting(nested)]
    pub cache: RemoteCacheConfig,

    #[setting(validate = validate::not_empty)]
    pub host: String,

    #[setting(nested)]
    pub tls: Option<RemoteTlsConfig>,
}
