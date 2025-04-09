use crate::config_struct;
use schematic::Config;

config_struct!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Config)]
    pub struct ExperimentsConfig {
        #[deprecated]
        #[setting(default = true)]
        pub action_pipeline_v2: bool,

        /// Disallow task relationships with different `runInCI` options.
        #[setting(default = true)]
        pub disallow_run_in_ci_mismatch: bool,

        /// Enable faster glob file system walking.
        pub faster_glob_walk: bool,

        /// Enable a faster and more accurate Git implementation.
        /// Supports submodules, subtrees, and worktrees.
        pub git_v2: bool,

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
