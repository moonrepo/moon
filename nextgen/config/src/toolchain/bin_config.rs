use schematic::Config;
use serde::Serialize;

#[derive(Clone, Config, Debug, Eq, PartialEq, Serialize)]
pub struct BinConfig {
    pub bin: String,

    pub force: bool,

    pub local: bool,

    pub version: Option<String>,
}

#[derive(Clone, Config, Debug, Eq, PartialEq, Serialize)]
#[config(serde(untagged, expecting = "expecting a bin name, or bin config object"))]
pub enum BinEntry {
    Name(String),
    #[setting(nested)]
    Config(BinConfig),
}

impl BinEntry {
    pub fn get_package_identifier(&self) -> String {
        match self {
            BinEntry::Name(name) => name.to_owned(),
            BinEntry::Config(cfg) => {
                if let Some(version) = cfg.version.as_ref() {
                    format!("{}@{version}", cfg.bin)
                } else {
                    cfg.bin.to_owned()
                }
            }
        }
    }
}
