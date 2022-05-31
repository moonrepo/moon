pub mod color;
mod logger;

pub use logger::Logger;

// Re-export so that consumers dont need to install these crates
pub use console::{measure_text_width, pad_str, pad_str_with, strip_ansi_codes};
pub use log::{debug, error, info, max_level, trace, warn, LevelFilter};

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
