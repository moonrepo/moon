use crate::extension_plugin::ExtensionPlugin;
use moon_common::Id;
use moon_config::ExtensionsConfig;
use moon_plugin::{
    MoonHostData, PluginLocator, PluginManifest, PluginRegistry, PluginType, PluginsConfig,
    serialize_config,
};
use starbase_utils::json::JsonValue;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;
use tracing::debug;

#[derive(Default, Debug)]
pub struct ExtensionRegistryConfig(Arc<ExtensionsConfig>);

impl PluginsConfig for ExtensionRegistryConfig {
    fn configure_manifest(
        &self,
        id: &Id,
        _host_data: &MoonHostData,
        manifest: &mut PluginManifest,
    ) -> miette::Result<()> {
        if let Some(cfg) = self.get_plugin_config(id) {
            let value = serialize_config(cfg.config.iter())?;

            debug!(
                extension_id = id.as_str(),
                config = %value,
                "Storing moon extension configuration",
            );

            manifest
                .config
                .insert("moon_extension_config".to_owned(), value);
        }

        Ok(())
    }

    fn get_ids(&self) -> Vec<&Id> {
        self.plugins.keys().collect()
    }

    fn get_json_config(&self, id: &Id) -> Option<JsonValue> {
        self.get_plugin_config(id).map(|cfg| cfg.to_json())
    }

    fn get_locator(&self, id: &Id) -> Option<&PluginLocator> {
        self.get_plugin_config(id)
            .and_then(|cfg| cfg.plugin.as_ref())
    }
}

impl Deref for ExtensionRegistryConfig {
    type Target = ExtensionsConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ExtensionRegistry(PluginRegistry<ExtensionRegistryConfig, ExtensionPlugin>);

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self(
            PluginRegistry::new(
                PluginType::Extension,
                MoonHostData::default(),
                ExtensionRegistryConfig::default(),
            )
            .unwrap(),
        )
    }
}

impl ExtensionRegistry {
    pub fn new(host_data: MoonHostData, config: Arc<ExtensionsConfig>) -> miette::Result<Self> {
        Ok(Self(PluginRegistry::new(
            PluginType::Extension,
            host_data,
            ExtensionRegistryConfig(config),
        )?))
    }

    pub fn create_config(&self, id: &Id) -> JsonValue {
        self.config_data.get_json_config(id).unwrap_or_default()
    }
}

impl Deref for ExtensionRegistry {
    type Target = PluginRegistry<ExtensionRegistryConfig, ExtensionPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
