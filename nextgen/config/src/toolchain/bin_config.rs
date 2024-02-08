use schematic::Config;
use serde::Serialize;

#[derive(Clone, Config, Debug, Eq, PartialEq, Serialize)]
pub struct BinConfig {
    /// Name of the binary, with optional version separated by `@`.
    pub bin: String,

    /// Force install the binary if it already exists.
    pub force: bool,

    /// Only install the binary locally, and not within CI.
    pub local: bool,

    /// For supported tools, a custom name to use.
    pub name: Option<String>,
}

#[derive(Clone, Config, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged, expecting = "expecting a bin name, or bin config object")]
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
