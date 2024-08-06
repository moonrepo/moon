use crate::extension_plugin::ExtensionPlugin;
use moon_config::ExtensionConfig;
use moon_plugin::{
    serialize_config, MoonEnvironment, PluginError, PluginId, PluginRegistry, PluginType,
    ProtoEnvironment,
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

impl ExtensionRegistry {
    pub fn new(moon_env: Arc<MoonEnvironment>, proto_env: Arc<ProtoEnvironment>) -> Self {
        Self {
            configs: FxHashMap::default(),
            registry: Arc::new(PluginRegistry::new(
                PluginType::Extension,
                moon_env,
                proto_env,
            )),
        }
    }

    pub async fn load(&self, id: &PluginId) -> miette::Result<()> {
        if self.is_registered(id) {
            return Ok(());
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
                    id = id.as_str(),
                    config = %value,
                    "Storing moon extension configuration",
                );

                manifest
                    .config
                    .insert("moon_extension_config".to_owned(), value);

                Ok(())
            })
            .await?;

        Ok(())
    }
}

impl Deref for ExtensionRegistry {
    type Target = PluginRegistry<ExtensionPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}
