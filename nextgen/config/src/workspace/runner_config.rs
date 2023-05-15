use moon_target::Target;
use schematic::{Config, ValidateError};

fn validate_cache_lifetime(value: &str) -> Result<(), ValidateError> {
    humantime::parse_duration(value)
        .map_err(|error| ValidateError::new(format!("invalid lifetime duration: {error}")))?;

    Ok(())
}

#[derive(Config)]
pub struct RunnerConfig {
    pub archivable_targets: Vec<Target>,

    #[setting(default_str = "7 days", validate = validate_cache_lifetime)]
    pub cache_lifetime: String,

    #[setting(default = true)]
    pub inherit_colors_for_piped_tasks: bool,

    pub log_running_command: bool,
}
