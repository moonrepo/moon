use moon_target::Target;
use schematic::Config;

#[derive(Config)]
pub struct RunnerConfig {
    pub archivable_targets: Vec<Target>,

    #[setting(default_str = "7 days")]
    pub cache_lifetime: String,

    #[setting(default = true)]
    pub inherit_colors_for_piped_tasks: bool,

    pub log_running_command: bool,
}
