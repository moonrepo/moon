use proto_core::PluginLocator;
use rustc_hash::FxHashMap;
use schematic::Config;

#[derive(Clone, Config, Debug, PartialEq)]
#[config(allow_unknown_fields)]
pub struct ExtensionConfig {
    #[setting(required)]
    pub plugin: Option<PluginLocator>,

    #[setting(flatten)]
    pub config: FxHashMap<String, serde_json::Value>,
}
