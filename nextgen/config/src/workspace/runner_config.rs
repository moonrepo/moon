use crate::types::Target;
use schematic::Config;

#[derive(Clone, Config, Debug)]
pub struct RunnerConfig {
    pub archivable_targets: Vec<Target>,

    #[setting(default = "7 days")]
    pub cache_lifetime: String,

    #[setting(default = true)]
    pub inherit_colors_for_piped_tasks: bool,

    pub log_running_command: bool,
}
