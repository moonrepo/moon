use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginType};

pub struct ExtensionPlugin {
    pub id: PluginId,
    plugin: PluginContainer,
}

impl ExtensionPlugin {
    pub fn execute(&self, args: Vec<String>, context: MoonContext) -> miette::Result<()> {
        self.plugin.call_func_without_output(
            "execute_extension",
            ExecuteExtensionInput { args, context },
        )?;

        Ok(())
    }
}

impl Plugin for ExtensionPlugin {
    fn new(id: PluginId, plugin: PluginContainer) -> Self {
        Self { id, plugin }
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }
}
