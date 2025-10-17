use crate::config_struct;
use schematic::Config;

config_struct!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Config)]
    pub struct ExperimentsConfig {
        /// Enable faster glob file system walking.
        #[setting(default = true)]
        pub faster_glob_walk: bool,
    }
);
