use super::bin_config::BinEntry;
use schematic::Config;
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

/// Configures and enables the Rust platform.
/// Docs: https://moonrepo.dev/docs/config/toolchain#rust
#[derive(Clone, Config, Debug)]
pub struct RustConfig {
    /// List of binaries to install into the environment using `cargo install`.
    #[setting(nested)]
    pub bins: Vec<BinEntry>,

    /// Rust components to automatically install.
    pub components: Vec<String>,

    /// Location of the WASM plugin to use for Rust support.
    pub plugin: Option<PluginLocator>,

    /// When `version` is defined, syncs the version to `rust-toolchain.toml`.
    pub sync_toolchain_config: bool,

    /// Rust targets to automatically install.
    pub targets: Vec<String>,

    /// The version of Rust to download, install, and run `cargo` tasks with.
    #[setting(env = "MOON_RUST_VERSION")]
    pub version: Option<UnresolvedVersionSpec>,
}
