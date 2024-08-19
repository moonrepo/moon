use async_trait::async_trait;
use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use std::fmt;
use tracing::instrument;

pub struct ExtensionPlugin {
    pub id: PluginId,
    plugin: PluginContainer,
}

impl ExtensionPlugin {
    #[instrument(skip(self, context))]
    pub async fn execute(&self, args: Vec<String>, context: MoonContext) -> miette::Result<()> {
        self.plugin
            .call_func_without_output("execute_extension", ExecuteExtensionInput { args, context })
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Plugin for ExtensionPlugin {
    async fn new(registration: PluginRegistration) -> miette::Result<Self> {
        Ok(Self {
            id: registration.id,
            plugin: registration.container,
        })
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }
}

impl fmt::Debug for ExtensionPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtensionPlugin")
            .field("id", &self.id)
            .finish()
    }
}
