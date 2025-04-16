use crate::{config_enum, config_struct};
use moon_common::Id;
use moon_target::Target;
use schematic::Config;

config_enum!(
    /// Toggles the state of actions within the pipeline.
    #[derive(Config)]
    #[serde(untagged)]
    pub enum PipelineActionSwitch {
        Default,
        Enabled(bool),
        Only(Vec<Id>),
    }
);

impl PipelineActionSwitch {
    pub fn is_disabled(&self) -> bool {
        match self {
            Self::Default => false,
            Self::Enabled(value) => !value,
            Self::Only(list) => list.is_empty(),
        }
    }

    pub fn is_enabled(&self, id: &Id) -> bool {
        match self {
            Self::Default => true,
            Self::Enabled(value) => *value,
            Self::Only(list) => list.contains(id),
        }
    }
}

impl From<bool> for PipelineActionSwitch {
    fn from(value: bool) -> Self {
        Self::Enabled(value)
    }
}

config_struct!(
    /// Configures aspects of the task runner (also known as the action pipeline).
    #[derive(Config)]
    pub struct PipelineConfig {
        /// List of target's for tasks without outputs, that should be
        /// cached and persisted.
        #[deprecated]
        pub archivable_targets: Vec<Target>,

        /// Automatically clean the cache after every task run.
        #[setting(default = true)]
        pub auto_clean_cache: bool,

        /// The lifetime in which task outputs will be cached.
        #[setting(default = "7 days")]
        pub cache_lifetime: String,

        /// Automatically inherit color settings for all tasks being ran.
        #[setting(default = true)]
        pub inherit_colors_for_piped_tasks: bool,

        /// Run the `InstallWorkspaceDeps` and `InstallProjectDeps` actions for
        /// each running task when changes to lockfiles and manifests are detected.
        #[setting(nested)]
        pub install_dependencies: PipelineActionSwitch,

        /// Threshold in milliseconds in which to force kill running child
        /// processes after the pipeline receives an external signal. A value
        /// of 0 will not kill the process and let them run to completion.
        #[setting(default = 2000)]
        pub kill_process_threshold: u32,

        /// Logs the task's command and arguments when running the task.
        pub log_running_command: bool,

        /// Run the `SyncProject` actions in the pipeline for each owning project
        /// of a running task.
        #[setting(nested)]
        pub sync_projects: PipelineActionSwitch,

        /// When creating `SyncProject` actions, recursively create a `SyncProject`
        /// action for each project dependency, and link them as a relationship.
        #[setting(default = true)]
        pub sync_project_dependencies: bool,

        /// Run the `SyncWorkspace` action before all actions in the pipeline.
        #[setting(default = true)]
        pub sync_workspace: bool,
    }
);
