use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(ValueEnum, Clone, Debug, Default, Display)]
pub enum CacheMode {
    Off,
    Read,
    #[default]
    ReadWrite,
    Write,
}

#[derive(ValueEnum, Clone, Debug, Default, Display)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

#[derive(
    ValueEnum, Clone, Copy, Debug, Deserialize, Display, Default, Eq, PartialEq, Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum TouchedStatus {
    Added,
    #[default]
    All,
    Deleted,
    Modified,
    Staged,
    Unstaged,
    Untracked,
}
