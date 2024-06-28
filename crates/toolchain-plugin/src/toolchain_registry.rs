use crate::toolchain_plugin::ToolchainPlugin;
use moon_config::ToolConfig;
use moon_plugin::{serialize_config, PluginId, PluginRegistry};
use proto_core::inject_proto_manifest_config;
use rustc_hash::FxHashMap;
use std::ops::Deref;

pub struct ToolchainRegistry {
    pub configs: FxHashMap<PluginId, ToolConfig>,
    pub registry: PluginRegistry<ToolchainPlugin>,
}

impl ToolchainRegistry {
    pub async fn load_all(&self) -> miette::Result<()> {
        let proto_env = &self.registry.proto_env;

        for (id, config) in &self.configs {
            self.registry
                .load_with_config(id, config.plugin.as_ref().unwrap(), move |manifest| {
                    manifest.config.insert(
                        "moon_toolchain_config".to_owned(),
                        serialize_config(config.config.iter())?,
                    );

                    inject_proto_manifest_config(id, proto_env, manifest)?;

                    Ok(())
                })
                .await?;
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
