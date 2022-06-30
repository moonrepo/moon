use clap::ArgEnum;
use strum_macros::Display;

#[derive(ArgEnum, Clone, Debug, Default, Display)]
pub enum CacheMode {
    Off,
    Read,
    #[default]
    Write,
}

#[derive(ArgEnum, Clone, Debug, Default, Display)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}
