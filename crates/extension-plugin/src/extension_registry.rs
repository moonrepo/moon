use crate::extension_plugin::ExtensionPlugin;
use moon_config::ExtensionConfig;
use moon_plugin::{
    serialize_config, PluginError, PluginHostData, PluginId, PluginRegistry, PluginType,
};
use rustc_hash::FxHashMap;
use std::ops::Deref;
use std::sync::Arc;
use tracing::trace;

#[derive(Debug)]
pub struct ExtensionRegistry {
    pub configs: FxHashMap<PluginId, ExtensionConfig>,
    registry: Arc<PluginRegistry<ExtensionPlugin>>,
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self {
            configs: FxHashMap::default(),
            registry: Arc::new(PluginRegistry::new(
                PluginType::Extension,
                PluginHostData::default(),
            )),
        }
    }
}

impl ExtensionRegistry {
    pub fn new(host_data: PluginHostData) -> Self {
        Self {
            configs: FxHashMap::default(),
            registry: Arc::new(PluginRegistry::new(PluginType::Extension, host_data)),
        }
    }

    pub async fn load(&self, id: &PluginId) -> miette::Result<Arc<ExtensionPlugin>> {
        if self.is_registered(id) {
            return self.get_instance(id).await;
        }

        let Some(config) = self.configs.get(id) else {
            return Err(PluginError::UnknownId {
                id: id.to_string(),
                ty: PluginType::Extension,
            }
            .into());
        };

        self.registry
            .load_with_config(&id, config.get_plugin_locator(), move |manifest| {
                let value = serialize_config(config.config.iter())?;

                trace!(
                    extension_id = id.as_str(),
                    config = %value,
                    "Storing moon extension configuration",
                );

                manifest
                    .config
                    .insert("moon_extension_config".to_owned(), value);

                Ok(())
            })
            .await?;

        self.get_instance(id).await
    }
}

impl Deref for ExtensionRegistry {
    type Target = PluginRegistry<ExtensionPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}
