use moon_common::cacheable;
use schematic::Config;

cacheable!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Clone, Config, Debug)]
    pub struct ExperimentsConfig {
        #[deprecated]
        #[setting(default = true)]
        pub action_pipeline_v2: bool,

        /// Disallow task relationships with different `runInCI` options.
        #[setting(default = true)]
        pub disallow_run_in_ci_mismatch: bool,

        /// Enable faster glob file system walking.
        pub faster_glob_walk: bool,

        #[deprecated]
        #[setting(default = true)]
        pub interweaved_task_inheritance: bool,

        #[deprecated]
        #[setting(default = true)]
        pub strict_project_aliases: bool,

        /// Disallow referencing the original ID of a renamed project when
        /// building the project graph.
        // #[setting(default = true)]
        pub strict_project_ids: bool,

        #[deprecated]
        #[setting(default = true)]
        pub task_output_boundaries: bool,
    }
);
