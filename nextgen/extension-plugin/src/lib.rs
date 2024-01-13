use moon_plugin::{Id, PluginContainer, PluginType};

pub struct ExtensionPlugin {
    pub id: Id,
    pub type_of: PluginType,

    plugin: PluginContainer<'static>,
}
