use crate::config_struct;
use crate::patterns::{merge_iter, merge_plugin_partials};
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{Config, validate};
use serde_json::Value;
use warpgate_api::{PluginLocator, UrlLocator};

config_struct!(
    /// Configures an individual extension.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ExtensionPluginConfig {
        /// Location of the WASM plugin to use.
        #[setting(required)]
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
        #[setting(
            default = "https://moonrepo.dev/schemas/extensions.json",
            rename = "$schema"
        )]
        pub schema: String,

        /// Extends one or many extensions configuration files.
        /// Supports a relative file path or a secure URL.
        /// @since 2.0.0
        #[setting(extend, validate = validate::extends_from)]
        pub extends: Option<schematic::ExtendsFrom>,

        /// Configures and integrates extensions into the system using
        /// a unique identifier.
        #[setting(flatten, nested, merge = merge_plugin_partials)]
        pub plugins: FxHashMap<Id, ExtensionPluginConfig>,
    }
);

impl ExtensionsConfig {
    pub fn inherit_default_plugins(&mut self) {
        for (id, extension) in default_extensions() {
            self.plugins.entry(id).or_insert(extension);
        }
    }

    pub fn get_plugin_config(&self, id: impl AsRef<str>) -> Option<&ExtensionPluginConfig> {
        self.plugins.get(id.as_ref())
    }

    pub fn is_plugin(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }
}

fn default_extensions() -> FxHashMap<Id, ExtensionPluginConfig> {
    FxHashMap::from_iter([
        (
            Id::raw("download"),
            ExtensionPluginConfig {
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/download_extension-v0.0.11/download_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
         (
            Id::raw("migrate-nx"),
            ExtensionPluginConfig {
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/migrate_nx_extension-v0.0.11/migrate_nx_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
        (
            Id::raw("migrate-turborepo"),
            ExtensionPluginConfig {
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/migrate_turborepo_extension-v0.1.8/migrate_turborepo_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
    ])
}
