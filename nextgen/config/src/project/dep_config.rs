use moon_common::Id;
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum DependencyScope {
        Development,
        Peer,
        #[default]
        Production,
    }
);

#[derive(Config)]
pub struct DependencyConfig {
    pub id: Id,
    pub scope: DependencyScope,
    pub via: Option<String>,
}
