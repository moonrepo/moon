use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;
use warpgate_api::{PluginLocator, UrlLocator};

/// Configures an individual extension.
#[derive(Clone, Config, Debug, PartialEq)]
#[config(allow_unknown_fields)]
pub struct ExtensionConfig {
    /// Location of the WASM plugin to use.
    #[setting(required)]
    pub plugin: Option<PluginLocator>,

    /// Arbitrary configuration that'll be passed to the WASM plugin.
    #[setting(flatten)]
    pub config: FxHashMap<String, serde_json::Value>,
}

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
                    url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_download_extension-v0.0.5/moon_download_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
         (
            Id::raw("migrate-nx"),
            ExtensionConfig {
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_migrate_nx_extension-v0.0.5/moon_migrate_nx_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
        (
            Id::raw("migrate-turborepo"),
            ExtensionConfig {
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_migrate_turborepo_extension-v0.1.2/moon_migrate_turborepo_extension.wasm".into()
                }))),
                config: FxHashMap::default(),
            },
        ),
    ])
}
