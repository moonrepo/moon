use rustc_hash::FxHashMap;
use schematic::Config;
use std::collections::BTreeMap;
use warpgate_api::PluginLocator;

#[derive(Clone, Config, Debug, PartialEq)]
#[config(allow_unknown_fields)]
pub struct ExtensionConfig {
    #[setting(required)]
    pub plugin: Option<PluginLocator>,

    #[setting(flatten)]
    pub config: BTreeMap<String, serde_json::Value>,
}

impl ExtensionConfig {
    pub fn get_plugin_locator(&self) -> &PluginLocator {
        self.plugin.as_ref().unwrap()
    }
}

pub fn default_extensions() -> FxHashMap<String, ExtensionConfig> {
    FxHashMap::from_iter([(
        "download".into(),
        ExtensionConfig {
            plugin: Some(PluginLocator::SourceUrl { url: "https://github.com/moonrepo/moon-extensions/releases/download/moon_download_extension-v0.0.1/moon_download_extension.wasm".into() }),
            config: BTreeMap::new(),
        },
    )])
}
