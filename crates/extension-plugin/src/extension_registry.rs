use crate::extension_plugin::ExtensionPlugin;
use moon_common::Id;
use moon_config::ExtensionConfig;
use moon_plugin::{
    PluginError, PluginHostData, PluginId, PluginRegistry, PluginType, serialize_config,
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

    pub fn inherit_configs(&mut self, configs: &FxHashMap<Id, ExtensionConfig>) {
        for (id, config) in configs {
            // Convert moon IDs to plugin IDs
            self.configs.insert(PluginId::raw(id), config.to_owned());
        }
    }

    pub async fn load<Id>(&self, id: Id) -> miette::Result<Arc<ExtensionPlugin>>
    where
        Id: AsRef<str>,
    {
        let id = PluginId::raw(id.as_ref());

        if self.is_registered(&id) {
            return self.get_instance(&id).await;
        }

        let Some(config) = self.configs.get(&id) else {
            return Err(PluginError::UnknownId {
                id: id.to_string(),
                ty: PluginType::Extension,
            }
            .into());
        };

        let ext_id = id.clone();

        self.registry
            .load(&id, config.get_plugin_locator(), move |manifest| {
                let value = serialize_config(config.config.iter())?;

                trace!(
                    extension_id = ext_id.as_str(),
                    config = %value,
                    "Storing moon extension configuration",
                );

                manifest
                    .config
                    .insert("moon_extension_config".to_owned(), value);

                Ok(())
            })
            .await?;

        self.get_instance(&id).await
    }
}

impl Deref for ExtensionRegistry {
    type Target = PluginRegistry<ExtensionPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}
