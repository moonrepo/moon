use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Error, Formatter};

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum CacheMode {
    Off,
    Read,
    #[default]
    ReadWrite,
    Write,
}

impl Display for CacheMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
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

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "{}",
            match self {
                LogLevel::Off => "off",
                LogLevel::Error => "error",
                LogLevel::Warn => "warn",
                LogLevel::Info => "info",
                LogLevel::Debug => "debug",
                LogLevel::Trace => "trace",
            }
        )?;

        Ok(())
    }
}

#[derive(ValueEnum, Clone, Copy, Debug, Deserialize, Default, Eq, PartialEq, Serialize)]
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

impl Display for TouchedStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "{}",
            match self {
                TouchedStatus::Added => "added",
                TouchedStatus::All => "all",
                TouchedStatus::Deleted => "deleted",
                TouchedStatus::Modified => "modified",
                TouchedStatus::Staged => "staged",
                TouchedStatus::Unstaged => "unstaged",
                TouchedStatus::Untracked => "untracked",
            }
        )?;

        Ok(())
    }
}
