use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;
use warpgate_api::PluginLocator;

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
                plugin: Some(PluginLocator::SourceUrl { url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_download_extension-v0.0.3/moon_download_extension.wasm".into() }),
                config: FxHashMap::default(),
            },
        ),
         (
            Id::raw("migrate-nx"),
            ExtensionConfig {
                plugin: Some(PluginLocator::SourceUrl { url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_migrate_nx_extension-v0.0.3/moon_migrate_nx_extension.wasm".into() }),
                config: FxHashMap::default(),
            },
        ),
        (
            Id::raw("migrate-turborepo"),
            ExtensionConfig {
                plugin: Some(PluginLocator::SourceUrl { url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_migrate_turborepo_extension-v0.1.0/moon_migrate_turborepo_extension.wasm".into() }),
                config: FxHashMap::default(),
            },
        ),
    ])
}
