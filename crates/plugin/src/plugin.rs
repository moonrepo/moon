use async_trait::async_trait;
use moon_env::MoonEnvironment;
use proto_core::ProtoEnvironment;
use std::fmt::Debug;
use std::sync::Arc;
use warpgate::{Id, PluginContainer, PluginLocator};

pub struct PluginRegistration {
    pub container: PluginContainer,
    pub id: Id,
    pub locator: PluginLocator,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,
}

#[derive(Clone, Copy, Debug)]
pub enum PluginType {
    Extension,
    Toolchain,
}

impl PluginType {
    pub fn get_dir_name(&self) -> &str {
        match self {
            PluginType::Extension => "extensions",
            PluginType::Toolchain => "toolchains",
        }
    }

    pub fn get_label(&self) -> &str {
        match self {
            PluginType::Extension => "extension",
            PluginType::Toolchain => "toolchain",
        }
    }
}

#[async_trait]
pub trait Plugin: Debug + Sized {
    async fn new(registration: PluginRegistration) -> miette::Result<Self>;
    fn get_type(&self) -> PluginType;
}
