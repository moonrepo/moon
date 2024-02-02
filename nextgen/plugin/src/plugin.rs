use moon_env::MoonEnvironment;
use proto_core::ProtoEnvironment;
use std::sync::Arc;
use warpgate::{Id, PluginContainer};

pub struct PluginRegistration {
    pub container: PluginContainer,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,
}

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

pub trait Plugin: Sized {
    fn new(id: Id, registration: PluginRegistration) -> miette::Result<Self>;
    fn get_type(&self) -> PluginType;
}
