use moon_common::cacheable;
use rustc_hash::FxHashMap;
use schematic::Config;
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

cacheable!(
    /// Configures an individual toolchain.
    #[derive(Clone, Config, Debug, PartialEq)]
    #[config(allow_unknown_fields)]
    pub struct ToolchainPluginConfig {
        /// Location of the WASM plugin to use.
        // #[setting(required)]
        pub plugin: Option<PluginLocator>,

        /// The version of the toolchain to download and install.
        pub version: Option<UnresolvedVersionSpec>,

        /// Arbitrary configuration that'll be passed to the WASM plugin.
        #[setting(flatten)]
        pub config: FxHashMap<String, serde_json::Value>,
    }
);
