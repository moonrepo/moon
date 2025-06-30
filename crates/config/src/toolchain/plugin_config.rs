use crate::{config_enum, config_struct};
use schematic::{Config, Schematic};
use serde_json::Value;
use std::collections::BTreeMap;
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

config_enum!(
    /// Strategy in which to inherit a version from `.prototools`.
    #[derive(Schematic)]
    #[serde(untagged)]
    pub enum ToolchainPluginVersionFrom {
        Enabled(bool),
        Id(String),
    }
);

impl Default for ToolchainPluginVersionFrom {
    fn default() -> Self {
        Self::Enabled(true)
    }
}

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

        /// Inherit the version from the root `.prototools`.
        /// When true, matches using the same ID, otherwise a
        /// string can be provided for a custom ID.
        pub version_from_prototools: ToolchainPluginVersionFrom,

        /// Arbitrary configuration that'll be passed to the WASM plugin.
        #[setting(flatten)]
        pub config: BTreeMap<String, Value>,
    }
);

impl ToolchainPluginConfig {
    pub fn to_json(&self) -> Value {
        let mut data = Value::Object(self.config.clone().into_iter().collect());

        if let Some(version) = &self.version {
            data["version"] = Value::String(version.to_string());
        }

        data
    }
}
