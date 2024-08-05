use moon_common::cacheable;
use schematic::Config;

cacheable!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Clone, Config, Debug)]
    pub struct ExperimentsConfig {
        #[deprecated]
        #[setting(default = true)]
        pub action_pipeline_v2: bool,

        #[deprecated]
        #[setting(default = true)]
        pub interweaved_task_inheritance: bool,

        #[deprecated]
        #[setting(default = true)]
        pub strict_project_aliases: bool,

        #[deprecated]
        #[setting(default = true)]
        pub task_output_boundaries: bool,
    }
);
