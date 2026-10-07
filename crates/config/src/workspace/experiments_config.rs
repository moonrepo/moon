use crate::config_struct;
use schematic::{Config, env};

config_struct!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Config)]
    pub struct ExperimentsConfig {
        /// Build the project and task graphs asynchronously.
        /// @since 2.2.0
        #[setting(default = true, env = "MOON_EXPERIMENT_ASYNC_GRAPH_BUILDING", parse_env = env::parse_bool)]
        pub async_graph_building: bool,

        /// Always use the task's `outputStyle` option, even when running
        /// the task as a primary target.
        /// @since 2.6.0
        #[setting(env = "MOON_EXPERIMENT_EXPLICIT_TASK_OUTPUT_STYLE", parse_env = env::parse_bool)]
        pub explicit_task_output_style: bool,
    }
);
