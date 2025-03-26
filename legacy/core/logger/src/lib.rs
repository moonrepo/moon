// Re-export so that consumers dont need to install these crates
pub use log::{LevelFilter, debug, error, info, max_level, trace, warn};

pub fn logging_enabled() -> bool {
    max_level() != LevelFilter::Off
}

pub trait Logable {
    /// Return a unique name for logging.
    fn get_log_target(&self) -> &str;
}

pub fn map_list<T, F>(files: &[T], fmt: F) -> String
where
    F: Fn(&T) -> String,
{
    files.iter().map(fmt).collect::<Vec<_>>().join(", ")
}
