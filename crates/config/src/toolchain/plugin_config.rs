use crate::config_struct;
use schematic::Config;
use std::collections::BTreeMap;
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

config_struct!(
    /// Configures an individual toolchain.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ToolchainPluginConfig {
        // TODO deprecate in v2
        #[doc(hidden)]
        pub disabled: bool,

        /// Location of the WASM plugin to use.
        pub plugin: Option<PluginLocator>,

        /// The version of the toolchain to download and install.
        pub version: Option<UnresolvedVersionSpec>,

        /// Arbitrary configuration that'll be passed to the WASM plugin.
        #[setting(flatten)]
        pub config: BTreeMap<String, serde_json::Value>,
    }
);
