use warpgate::{Id, PluginContainer};

#[derive(Clone, Copy, Debug)]
pub enum PluginType {
    Extension,
    Platform,
}

impl PluginType {
    pub fn get_dir_name(&self) -> &str {
        match self {
            PluginType::Extension => "extensions",
            PluginType::Platform => "platforms",
        }
    }

    pub fn get_label(&self) -> &str {
        match self {
            PluginType::Extension => "extension",
            PluginType::Platform => "platform",
        }
    }
}

pub trait Plugin {
    fn new(id: Id, plugin: PluginContainer) -> Self;
    fn get_type(&self) -> PluginType;
}
