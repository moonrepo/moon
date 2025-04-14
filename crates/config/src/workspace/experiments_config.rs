use crate::config_struct;
use schematic::Config;

config_struct!(
    /// Configures experiments across the entire moon workspace.
    #[derive(Config)]
    pub struct ExperimentsConfig {
        #[deprecated]
        #[setting(default = true)]
        pub action_pipeline_v2: bool,

        #[deprecated]
        #[setting(default = true)]
        pub disallow_run_in_ci_mismatch: bool,

        /// Enable faster glob file system walking.
        #[setting(default = true)]
        pub faster_glob_walk: bool,

        /// Enable a faster and more accurate Git implementation.
        /// Supports submodules, subtrees, and worktrees.
        #[setting(default = true)]
        pub git_v2: bool,

        #[deprecated]
        #[setting(default = true)]
        pub interweaved_task_inheritance: bool,

        #[deprecated]
        #[setting(default = true)]
        pub strict_project_aliases: bool,

        #[deprecated]
        #[setting(default = true)]
        pub strict_project_ids: bool,

        #[deprecated]
        #[setting(default = true)]
        pub task_output_boundaries: bool,
    }
);
