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
}

pub trait Plugin
where
    Self: Sized,
{
    fn new(id: Id, plugin: PluginContainer) -> miette::Result<Self>;
    fn get_type(&self) -> PluginType;
}
