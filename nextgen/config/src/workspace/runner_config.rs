use moon_target::Target;
use schematic::Config;
use serde::Serialize;

#[derive(Config, Serialize)]
pub struct RunnerConfig {
    pub archivable_targets: Vec<Target>,

    #[setting(default = "7 days")]
    pub cache_lifetime: String,

    #[setting(default = true)]
    pub inherit_colors_for_piped_tasks: bool,

    pub log_running_command: bool,
}
