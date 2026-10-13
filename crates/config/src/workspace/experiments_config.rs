use crate::config_struct;
use schematic::Config;

config_struct!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Config)]
    #[serde(default)]
    pub struct ExperimentsConfig {}
);
