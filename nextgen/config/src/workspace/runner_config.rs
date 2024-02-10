use moon_target::Target;
use schematic::Config;

/// Configures aspects of the task runner (also known as the action pipeline).
#[derive(Clone, Config, Debug)]
pub struct RunnerConfig {
    /// List of target's for tasks without outputs, that should be
    /// cached and persisted.
    pub archivable_targets: Vec<Target>,

    /// The lifetime in which task outputs will be cached.
    #[setting(default = "7 days")]
    pub cache_lifetime: String,

    /// Automatically inherit color settings for all tasks being ran.
    #[setting(default = true)]
    pub inherit_colors_for_piped_tasks: bool,

    /// Logs the task's command and arguments when running the task.
    pub log_running_command: bool,
}
