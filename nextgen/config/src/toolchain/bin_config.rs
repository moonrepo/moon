use schematic::Config;
use serde::Serialize;

#[derive(Clone, Config, Debug, Eq, PartialEq, Serialize)]
pub struct BinConfig {
    pub bin: String,

    pub force: bool,

    pub local: bool,

    pub name: Option<String>,
}

#[derive(Clone, Config, Debug, Eq, PartialEq, Serialize)]
#[config(serde(untagged, expecting = "expecting a bin name, or bin config object"))]
pub enum BinEntry {
    Name(String),
    #[setting(nested)]
    Config(BinConfig),
}

impl BinEntry {
    pub fn get_name(&self) -> &str {
        match self {
            BinEntry::Name(name) => name,
            BinEntry::Config(cfg) => &cfg.bin,
        }
    }
}
