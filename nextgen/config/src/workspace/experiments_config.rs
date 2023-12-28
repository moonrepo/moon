use moon_common::cacheable;
use schematic::Config;

cacheable!(
    #[derive(Clone, Config, Debug)]
    pub struct ExperimentsConfig {
        #[deprecated]
        #[setting(default = true)]
        pub interweaved_task_inheritance: bool,

        #[setting(default = true)]
        pub strict_project_aliases: bool,

        #[deprecated]
        #[setting(default = true)]
        pub task_output_boundaries: bool,
    }
);
