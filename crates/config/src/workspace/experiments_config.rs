use crate::config_struct;
use schematic::{Config, env};

config_struct!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Config)]
    pub struct ExperimentsConfig {
        /// Track and determine affected projects and tasks asynchronously.
        /// @since 2.2.0
        #[setting(env = "MOON_EXPERIMENT_ASYNC_AFFECTED_TRACKING", parse_env = env::parse_bool)]
        pub async_affected_tracking: bool,

        /// Build the project and task graphs asynchronously.
        /// @since 2.2.0
        #[setting(env = "MOON_EXPERIMENT_ASYNC_GRAPH_BUILDING", parse_env = env::parse_bool)]
        pub async_graph_building: bool,

        /// Store task outputs in a local CAS (content-addressable storage) cache.
        /// @since 2.3.0
        #[setting(env = "MOON_EXPERIMENT_CAS_OUTPUTS_CACHE", parse_env = env::parse_bool)]
        pub cas_outputs_cache: bool,

        /// Use native file hashing instead of using the VCS.
        /// @since 2.3.0
        #[setting(env = "MOON_EXPERIMENT_NATIVE_FILE_HASHING", parse_env = env::parse_bool)]
        pub native_file_hashing: bool,
    }
);
