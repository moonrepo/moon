use crate::config_struct;
use schematic::Config;

config_struct!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Config)]
    pub struct ExperimentsConfig {
        /// Track and determine affected projects and tasks asynchronously.
        pub async_affected_tracking: bool,

        /// Build the project and task graphs asynchronously.
        pub async_graph_building: bool,
    }
);
