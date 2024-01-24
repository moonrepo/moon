use moon_pdk_api::extension::*;
use moon_plugin::{Id, Plugin, PluginContainer, PluginType};

pub struct ExtensionPlugin {
    pub id: Id,
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
        Self { id, plugin }
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }
}
