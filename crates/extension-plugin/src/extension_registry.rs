use crate::extension_plugin::ExtensionPlugin;
use moon_plugin::{MoonEnvironment, PluginRegistry, PluginType, ProtoEnvironment};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub struct ExtensionRegistry {
    registry: Arc<PluginRegistry<ExtensionPlugin>>,
}

impl ExtensionRegistry {
    pub fn new(moon_env: Arc<MoonEnvironment>, proto_env: Arc<ProtoEnvironment>) -> Self {
        Self {
            registry: Arc::new(PluginRegistry::new(
                PluginType::Extension,
                moon_env,
                proto_env,
            )),
        }
    }
}

impl Deref for ExtensionRegistry {
    type Target = PluginRegistry<ExtensionPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}
