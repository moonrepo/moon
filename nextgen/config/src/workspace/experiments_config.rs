use moon_common::cacheable;
use schematic::{env, Config};

cacheable!(
    #[derive(Clone, Config, Debug)]
    pub struct ExperimentsConfig {
        #[deprecated]
        #[setting(default = true)]
        pub interweaved_task_inheritance: bool,

        #[setting(default = true)]
        pub strict_project_aliases: bool,

        #[deprecated]
        #[setting(default = true, env = "MOON_DISABLE_OVERLAPPING_OUTPUTS", parse_env = env::parse_bool)]
        pub task_output_boundaries: bool,
    }
);
