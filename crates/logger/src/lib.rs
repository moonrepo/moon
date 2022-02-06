pub mod color;
mod logger;

pub use logger::Logger;

// Re-export so that consumers dont need to install the log crate
pub use log::{debug, error, info, max_level, trace, warn, LevelFilter};

pub fn logging_enabled() -> bool {
    max_level() != LevelFilter::Off
}
