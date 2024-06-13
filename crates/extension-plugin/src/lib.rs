use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use tracing::instrument;

pub struct ExtensionPlugin {
    pub id: PluginId,
    plugin: PluginContainer,
}

impl ExtensionPlugin {
    #[instrument(skip(self, context))]
    pub fn execute(&self, args: Vec<String>, context: MoonContext) -> miette::Result<()> {
        self.plugin.call_func_without_output(
            "execute_extension",
            ExecuteExtensionInput { args, context },
        )?;

        Ok(())
    }
}

impl Plugin for ExtensionPlugin {
    fn new(registration: PluginRegistration) -> miette::Result<Self> {
        Ok(Self {
            id: registration.id,
            plugin: registration.container,
        })
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }
}
