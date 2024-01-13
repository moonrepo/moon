pub enum PluginType {
    Extension,
    Platform,
}

pub trait Plugin {
    fn get_type(&self) -> PluginType;
}
