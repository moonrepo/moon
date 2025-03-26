use moon_env_var::GlobalEnvBag;
use std::fmt;
use tracing::warn;

static mut LOGGED_WARNING: bool = false;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CacheMode {
    Off,
    Read,
    #[default]
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
            unknown => {
                // We only want to show this once, not everytime the function is called
                unsafe {
                    if !LOGGED_WARNING {
                        LOGGED_WARNING = true;

                        warn!(
                            "Unknown MOON_CACHE environment variable value \"{}\", falling back to read-write mode",
                            unknown
                        );
                    }
                }

                CacheMode::ReadWrite
            }
        }
    }
}

impl fmt::Display for CacheMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                CacheMode::Off => "off",
                CacheMode::Read => "read",
                CacheMode::ReadWrite => "read-write",
                CacheMode::Write => "write",
            }
        )?;

        Ok(())
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
    if let Some(var) = GlobalEnvBag::instance().get("MOON_CACHE") {
        return CacheMode::from(var);
    }

    CacheMode::ReadWrite
}
