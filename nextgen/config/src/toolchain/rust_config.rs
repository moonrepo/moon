use super::bin_config::BinEntry;
use crate::validate::validate_semver;
use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#rust
#[derive(Clone, Config, Debug)]
pub struct RustConfig {
    #[setting(nested)]
    pub bins: Vec<BinEntry>,

    pub plugin: Option<String>,

    pub sync_toolchain_config: bool,

    #[setting(env = "MOON_RUST_VERSION", validate = validate_semver)]
    pub version: Option<String>,
}
