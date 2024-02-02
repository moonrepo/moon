use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;
use warpgate_api::PluginLocator;

#[derive(Clone, Config, Debug, PartialEq)]
#[config(allow_unknown_fields)]
pub struct ExtensionConfig {
    #[setting(required)]
    pub plugin: Option<PluginLocator>,

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
                plugin: Some(PluginLocator::SourceUrl { url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_download_extension-v0.0.2/moon_download_extension.wasm".into() }),
                config: FxHashMap::default(),
            },
        ),
        (
            Id::raw("migrate-turborepo"),
            ExtensionConfig {
                plugin: Some(PluginLocator::SourceUrl { url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_migrate_turborepo_extension-v0.0.2/moon_migrate_turborepo_extension.wasm".into() }),
                config: FxHashMap::default(),
            },
        ),
    ])
}
