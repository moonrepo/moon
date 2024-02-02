use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::Tool;
use std::sync::Arc;

pub struct PlatformPlugin {
    pub id: PluginId,
    plugin: Arc<PluginContainer>,
    tool: Tool,
}

impl PlatformPlugin {}

impl Plugin for PlatformPlugin {
    fn new(id: PluginId, registration: PluginRegistration) -> miette::Result<Self> {
        let plugin = Arc::new(registration.container);

        Ok(Self {
            tool: Tool::new(
                id.clone(),
                Arc::clone(&registration.proto_env),
                Arc::clone(&plugin),
            )?,
            id,
            plugin,
        })
    }

    fn get_type(&self) -> PluginType {
        PluginType::Platform
    }
}
