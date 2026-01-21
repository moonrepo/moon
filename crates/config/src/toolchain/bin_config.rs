use crate::{config_enum, config_struct};
use schematic::{Config, validate};

config_struct!(
    /// Configures to a tool-specific global binary to install.
    #[derive(Config)]
    pub struct BinConfig {
        /// Name of the binary, with optional version separated by `@`.
        #[setting(validate = validate::not_empty)]
        pub bin: String,

        /// Force install the binary if it already exists.
        pub force: bool,

        /// Only install the binary locally, and not within CI.
        pub local: bool,

        /// For supported tools, a custom name to use.
        pub name: Option<String>,
    }
);

config_enum!(
    /// Configures to a tool-specific global binary to install.
    #[derive(Config)]
    #[serde(untagged)]
    pub enum BinEntry {
        /// Name of the binary to install.
        Name(String),

        /// Expanded configuration for the binary to install.
        #[setting(nested)]
        Object(BinConfig),
    }
);

impl BinEntry {
    pub fn get_name(&self) -> &str {
        match self {
            BinEntry::Name(name) => name,
            BinEntry::Object(cfg) => &cfg.bin,
        }
    }
}
