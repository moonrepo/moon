use crate::toolchain_plugin::ToolchainPlugin;
use miette::IntoDiagnostic;
use moon_config::ToolchainPluginConfig;
use moon_plugin::{PluginHostData, PluginId, PluginRegistry, PluginType, serialize_config};
use proto_core::inject_proto_manifest_config;
use rustc_hash::FxHashMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, trace};

#[derive(Debug)]
pub struct ToolchainRegistry {
    pub configs: FxHashMap<PluginId, ToolchainPluginConfig>,
    registry: Arc<PluginRegistry<ToolchainPlugin>>,
}

impl ToolchainRegistry {
    pub fn new(host_data: PluginHostData) -> Self {
        Self {
            configs: FxHashMap::default(),
            registry: Arc::new(PluginRegistry::new(PluginType::Toolchain, host_data)),
        }
    }

    pub fn get_plugin_ids(&self) -> Vec<&PluginId> {
        self.configs.keys().collect()
    }

    pub fn has_plugins(&self) -> bool {
        !self.configs.is_empty()
    }

    pub async fn load_all(&self) -> miette::Result<()> {
        if !self.has_plugins() {
            return Ok(());
        }

        debug!("Loading all toolchain plugins");

        let mut set = JoinSet::new();

        for (id, config) in self.configs.clone() {
            let registry = Arc::clone(&self.registry);

            set.spawn(async move {
                registry
                    .load_with_config(&id, config.plugin.as_ref().unwrap(), |manifest| {
                        let value = serialize_config(config.config.iter())?;

                        trace!(
                            toolchain_id = id.as_str(),
                            config = %value,
                            "Storing moon toolchain configuration",
                        );

                        manifest
                            .config
                            .insert("moon_toolchain_config".to_owned(), value);

                        inject_proto_manifest_config(&id, &registry.host_data.proto_env, manifest)?;

                        Ok(())
                    })
                    .await
            });
        }

        while let Some(result) = set.join_next().await {
            result.into_diagnostic()??;
        }

        Ok(())
    }
}

impl Deref for ToolchainRegistry {
    type Target = PluginRegistry<ToolchainPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}
