use crate::config_struct;
use crate::patterns::{merge_iter, merge_plugin_partials};
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{Config, validate};
use serde_json::Value;
use warpgate_api::PluginLocator;

config_struct!(
    /// Configures an individual extension.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ExtensionPluginConfig {
        /// Location of the WASM plugin to use.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub plugin: Option<PluginLocator>,

        /// Arbitrary configuration that'll be passed to the WASM plugin.
        #[setting(flatten, merge = merge_iter)]
        pub config: FxHashMap<String, serde_json::Value>,
    }
);

impl ExtensionPluginConfig {
    pub fn get_plugin_locator(&self) -> &PluginLocator {
        self.plugin.as_ref().unwrap()
    }

    pub fn to_json(&self) -> Value {
        Value::Object(self.config.clone().into_iter().collect())
    }
}

config_struct!(
    /// Configures all extensions.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ExtensionsConfig {
        #[setting(default = "./cache/schemas/extensions.json", rename = "$schema")]
        pub schema: String,

        /// Extends one or many extensions configuration files.
        /// Supports a relative file path or a secure URL.
        /// @since 2.0.0
        #[setting(extend, validate = validate::extends_from)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub extends: Option<schematic::ExtendsFrom>,

        /// Configures and integrates extensions into the system using
        /// a unique identifier.
        #[setting(flatten, nested, merge = merge_plugin_partials)]
        pub plugins: FxHashMap<Id, ExtensionPluginConfig>,
    }
);

impl ExtensionsConfig {
    pub fn get_plugin_config(&self, id: impl AsRef<str>) -> Option<&ExtensionPluginConfig> {
        self.plugins.get(id.as_ref())
    }

    pub fn should_invalidate(&self, other: &Self) -> bool {
        self.plugins != other.plugins
    }
}
