use crate::config_struct;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;
use warpgate_api::{PluginLocator, UrlLocator};

config_struct!(
    /// Configures an individual extension.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ExtensionConfig {
        /// Location of the WASM plugin to use.
        #[setting(required)]
        pub plugin: Option<PluginLocator>,

        /// Arbitrary configuration that'll be passed to the WASM plugin.
        #[setting(flatten)]
        pub config: FxHashMap<String, serde_json::Value>,
    }
);

impl ExtensionConfig {
    pub fn get_plugin_locator(&self) -> &PluginLocator {
        self.plugin.as_ref().unwrap()
    }
}

pub(crate) fn default_extensions() -> FxHashMap<Id, ExtensionConfig> {
    FxHashMap::from_iter([
        (
            Id::raw("download"),
            ExtensionConfig {
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/download_extension-v0.0.9/download_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
         (
            Id::raw("migrate-nx"),
            ExtensionConfig {
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/migrate_nx_extension-v0.0.9/migrate_nx_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
        (
            Id::raw("migrate-turborepo"),
            ExtensionConfig {
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/migrate_turborepo_extension-v0.1.6/migrate_turborepo_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
    ])
}
