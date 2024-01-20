use moon_pdk_api::extension::*;
use moon_plugin::{Id, Plugin, PluginContainer, PluginType};

pub struct ExtensionPlugin {
    pub id: Id,
    pub type_of: PluginType,

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
    fn new(id: Id, plugin: PluginContainer) -> Self {
        Self {
            type_of: PluginType::Extension,
            plugin,
            id,
        }
    }

    fn get_type(&self) -> PluginType {
        self.type_of
    }
}
