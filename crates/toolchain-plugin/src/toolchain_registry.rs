use crate::toolchain_plugin::ToolchainPlugin;
use moon_common::Id;
use moon_config::{ProjectConfig, ProjectToolchainEntry, ToolchainsConfig};
use moon_plugin::{
    MoonHostData, PluginManifest, PluginRegistry, PluginType, PluginsConfig, serialize_config,
};
use proto_core::{PluginLocator, ToolContext, inject_proto_manifest_config};
use starbase_utils::json::{self, JsonValue};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;
use tracing::debug;

#[derive(Default, Debug)]
pub struct ToolchainRegistryConfig(Arc<ToolchainsConfig>);

impl PluginsConfig for ToolchainRegistryConfig {
    fn configure_manifest(
        &self,
        id: &Id,
        host_data: &MoonHostData,
        manifest: &mut PluginManifest,
    ) -> miette::Result<()> {
        if let Some(cfg) = self.get_plugin_config(id) {
            let value = serialize_config(cfg.config.iter())?;

            debug!(
                toolchain_id = id.as_str(),
                config = %value,
                "Storing moon toolchain configuration",
            );

            manifest
                .config
                .insert("moon_toolchain_config".to_owned(), value);

            inject_proto_manifest_config(
                &ToolContext::new(id.clone()),
                &host_data.proto_env,
                manifest,
            )?;
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

impl Deref for ToolchainRegistryConfig {
    type Target = ToolchainsConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct ToolchainRegistry(PluginRegistry<ToolchainRegistryConfig, ToolchainPlugin>);

impl Default for ToolchainRegistry {
    fn default() -> Self {
        Self(
            PluginRegistry::new(
                PluginType::Toolchain,
                MoonHostData::default(),
                ToolchainRegistryConfig::default(),
            )
            .unwrap(),
        )
    }
}

impl ToolchainRegistry {
    pub fn new(host_data: MoonHostData, config: Arc<ToolchainsConfig>) -> miette::Result<Self> {
        Ok(Self(PluginRegistry::new(
            PluginType::Toolchain,
            host_data,
            ToolchainRegistryConfig(config),
        )?))
    }

    pub fn create_config(&self, id: &Id) -> JsonValue {
        self.config_data.get_json_config(id).unwrap_or_default()
    }

    pub fn create_merged_config(&self, id: &Id, project_config: &ProjectConfig) -> JsonValue {
        let mut data = self.create_config(id);

        if let Some(ProjectToolchainEntry::Object(leaf_config)) =
            project_config.toolchains.get_plugin_config(id)
        {
            let next = leaf_config.to_json();

            data = json::merge(&data, &next);
        }

        data
    }
}

impl Deref for ToolchainRegistry {
    type Target = PluginRegistry<ToolchainRegistryConfig, ToolchainPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
