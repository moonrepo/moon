use moon_common::Id;
use schematic::{config_enum, Config};
use strum::Display;

config_enum!(
    #[derive(Default, Display)]
    pub enum DependencyScope {
        #[strum(serialize = "development")]
        Development,

        #[strum(serialize = "peer")]
        Peer,

        #[default]
        #[strum(serialize = "production")]
        Production,
    }
);

#[derive(Config)]
pub struct DependencyConfig {
    pub id: Id,
    pub scope: DependencyScope,
    pub via: Option<String>,
}
