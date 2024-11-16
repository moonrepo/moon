use schematic::{validate, Config};
use std::path::PathBuf;

#[derive(Clone, Config, Debug)]
pub struct RemoteTlsConfig {
    pub domain_name: String,
    pub pem_file: PathBuf,
}

#[derive(Clone, Config, Debug)]
pub struct RemoteConfig {
    #[setting(validate = validate::not_empty)]
    pub host: String,

    #[setting(nested)]
    pub tls: Option<RemoteTlsConfig>,
}
