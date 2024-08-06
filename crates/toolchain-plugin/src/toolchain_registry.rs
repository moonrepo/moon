use crate::toolchain_plugin::ToolchainPlugin;
use miette::IntoDiagnostic;
use moon_config::ToolConfig;
use moon_plugin::{
    serialize_config, MoonEnvironment, PluginId, PluginRegistry, PluginType, ProtoEnvironment,
};
use proto_core::inject_proto_manifest_config;
use rustc_hash::FxHashMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, trace};

#[derive(Debug)]
pub struct ToolchainRegistry {
    pub configs: FxHashMap<PluginId, ToolConfig>,
    registry: Arc<PluginRegistry<ToolchainPlugin>>,
}

impl ToolchainRegistry {
    pub fn new(moon_env: Arc<MoonEnvironment>, proto_env: Arc<ProtoEnvironment>) -> Self {
        Self {
            configs: FxHashMap::default(),
            registry: Arc::new(PluginRegistry::new(
                PluginType::Toolchain,
                moon_env,
                proto_env,
            )),
        }
    }

    pub async fn load_all(&self) -> miette::Result<()> {
        if self.configs.is_empty() {
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
                            id = id.as_str(),
                            config = %value,
                            "Storing moon toolchain configuration",
                        );

                        manifest
                            .config
                            .insert("moon_toolchain_config".to_owned(), value);

                        inject_proto_manifest_config(&id, &registry.proto_env, manifest)?;

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
