use moon_logger::warn;
use std::env;

pub const LOG_TARGET: &str = "moon:cache";

static mut LOGGED_WARNING: bool = false;

pub enum CacheLevel {
    Off,
    Read,
    ReadWrite,
    Write,
}

impl From<String> for CacheLevel {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "off" => CacheLevel::Off,
            "read" => CacheLevel::Read,
            "read-write" => CacheLevel::ReadWrite,
            "write" => CacheLevel::Write,
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

                CacheLevel::ReadWrite
            }
        }
    }
}

impl CacheLevel {
    pub fn is_readable(&self) -> bool {
        matches!(&self, CacheLevel::Read | CacheLevel::ReadWrite)
    }

    pub fn is_writable(&self) -> bool {
        matches!(&self, CacheLevel::Write | CacheLevel::ReadWrite)
    }
}

pub fn get_cache_level() -> CacheLevel {
    if let Ok(var) = env::var("MOON_CACHE") {
        return CacheLevel::from(var);
    }

    CacheLevel::ReadWrite
}
