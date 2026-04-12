use crate::config_struct;
use schematic::{Config, env};

config_struct!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Config)]
    pub struct ExperimentsConfig {
        /// Track and determine affected projects and tasks asynchronously.
        #[setting(env = "MOON_EXPERIMENT_ASYNC_AFFECTED_TRACKING", parse_env = env::parse_bool)]
        pub async_affected_tracking: bool,

        /// Build the project and task graphs asynchronously.
        #[setting(env = "MOON_EXPERIMENT_ASYNC_GRAPH_BUILDING", parse_env = env::parse_bool)]
        pub async_graph_building: bool,
    }
);
