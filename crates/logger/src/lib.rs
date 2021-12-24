pub mod color;
mod logger;

pub use logger::Logger;
// Re-export macros so that consumers dont need to install the log crate
pub use log::{debug, error, info, trace, warn};
