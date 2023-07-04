use moon_config::LanguageType;
use starbase_events::Event;
use std::path::PathBuf;

#[derive(Debug)]
pub struct DetectLanguageEvent {
    pub project_root: PathBuf,
}

impl Event for DetectLanguageEvent {
    type Value = LanguageType;
}
