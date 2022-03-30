use clap::ArgEnum;
use strum_macros::Display;

#[derive(ArgEnum, Clone, Display)]
pub enum CacheMode {
    Off,
    Read,
    Write,
}

impl Default for CacheMode {
    fn default() -> Self {
        CacheMode::Write
    }
}

#[derive(ArgEnum, Clone, Display)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}
