use schematic::Config;

#[derive(Clone, Config, Debug)]
pub struct BinConfig {
    pub bin: String,

    pub force: bool,

    pub local: bool,

    pub version: Option<String>,
}

#[derive(Clone, Config, Debug)]
#[config(serde(untagged, expecting = "expecting a bin name, or bin config object"))]
pub enum BinEntry {
    Name(String),
    #[setting(nested)]
    Config(BinConfig),
}
