use super::bin_config::BinEntry;
use proto_core::{PluginLocator, UnresolvedVersionSpec};
use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#rust
#[derive(Clone, Config, Debug)]
pub struct RustConfig {
    #[setting(nested)]
    pub bins: Vec<BinEntry>,

    pub components: Vec<String>,

    pub plugin: Option<PluginLocator>,

    pub sync_toolchain_config: bool,

    pub targets: Vec<String>,

    #[setting(env = "MOON_RUST_VERSION")]
    pub version: Option<UnresolvedVersionSpec>,
}
