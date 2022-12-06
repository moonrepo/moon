use moon_logger::warn;
use std::env;

pub const LOG_TARGET: &str = "moon:cache";

static mut LOGGED_WARNING: bool = false;

pub enum CacheMode {
    Off,
    Read,
    ReadWrite,
    Write,
}

impl From<String> for CacheMode {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "off" => CacheMode::Off,
            "read" => CacheMode::Read,
            "read-write" => CacheMode::ReadWrite,
            "write" => CacheMode::Write,
            val => {
                // We only want to show this once, not everytime the function is called
                unsafe {
                    if !LOGGED_WARNING {
                        LOGGED_WARNING = true;

                        warn!(
                            target: LOG_TARGET,
                            "Unknown MOON_CACHE environment variable value \"{}\", falling back to read-write mode",
                            val
                        );
                    }
                }

                CacheMode::ReadWrite
            }
        }
    }
}

impl CacheMode {
    pub fn is_readable(&self) -> bool {
        matches!(&self, CacheMode::Read | CacheMode::ReadWrite)
    }

    pub fn is_read_only(&self) -> bool {
        matches!(&self, CacheMode::Read)
    }

    pub fn is_writable(&self) -> bool {
        matches!(&self, CacheMode::Write | CacheMode::ReadWrite)
    }

    pub fn is_write_only(&self) -> bool {
        matches!(&self, CacheMode::Write)
    }
}

pub fn get_cache_mode() -> CacheMode {
    if let Ok(var) = env::var("MOON_CACHE") {
        return CacheMode::from(var);
    }

    CacheMode::ReadWrite
}
