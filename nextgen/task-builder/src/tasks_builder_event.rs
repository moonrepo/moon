use moon_config::PlatformType;
use starbase_events::Event;

#[derive(Debug)]
pub struct DetectPlatformEvent {
    pub enabled_platforms: Vec<PlatformType>,
    pub task_command: String,
}

impl Event for DetectPlatformEvent {
    type Value = PlatformType;
}
