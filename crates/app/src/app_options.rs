use clap::ValueEnum;
use moon_common::is_ci;
use moon_console::Level;
use std::fmt;
use std::str::FromStr;

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
}

impl fmt::Display for AppTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::Dark => "dark",
                Self::Light => "light",
            }
        )
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
    Verbose,
}

impl LogLevel {
    pub fn is_verbose(&self) -> bool {
        matches!(self, Self::Verbose)
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::Off => "off",
                Self::Error => "error",
                Self::Warn => "warn",
                Self::Info => "info",
                Self::Debug => "debug",
                // Must map to tracing levels
                Self::Trace | Self::Verbose => "trace",
            }
        )
    }
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum SummaryLevel {
    None,
    Minimal,
    Normal,
    #[default]
    Detailed,
}

impl SummaryLevel {
    pub fn to_level(&self) -> Level {
        match self {
            Self::None => Level::Zero,
            Self::Minimal => Level::One,
            Self::Normal => Level::Three,
            Self::Detailed => Level::Three,
        }
    }
}

impl fmt::Display for SummaryLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::Minimal => "minimal",
                Self::Normal => "normal",
                Self::Detailed => "detailed",
            }
        )
    }
}

#[derive(Clone, Debug)]
pub enum AffectedOption {
    Bool(bool),
    String(String),
}

impl AffectedOption {
    pub fn get_base(&self) -> Option<String> {
        if let Self::String(inner) = self {
            for part in inner.split(',') {
                if part.contains("local") || part.contains("remote") {
                    continue;
                } else if let Some((base, _)) = part.split_once(':') {
                    return Some(base.into());
                } else {
                    return Some(part.into());
                }
            }
        }

        None
    }

    pub fn get_head(&self) -> Option<String> {
        if let Self::String(inner) = self {
            for part in inner.split(',') {
                if let Some((_, head)) = part.split_once(':') {
                    return Some(head.into());
                }
            }
        }

        None
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Bool(inner) => *inner,
            Self::String(_) => true,
        }
    }

    pub fn is_local(&self) -> bool {
        let mut local = !is_ci();

        if let Self::String(inner) = self {
            for part in inner.split(',') {
                if part == "local" || part == "!remote" {
                    local = true;
                } else if part == "remote" || part == "!local" {
                    local = false;
                }
            }
        }

        local
    }
}

impl fmt::Display for AffectedOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::Bool(inner) =>
                    if *inner {
                        "true"
                    } else {
                        "false"
                    },
                Self::String(inner) => inner.as_str(),
            }
        )
    }
}

impl From<String> for AffectedOption {
    fn from(value: String) -> Self {
        Self::from_str(&value).unwrap()
    }
}

impl FromStr for AffectedOption {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(if value == "true" {
            Self::Bool(true)
        } else if value == "false" {
            Self::Bool(false)
        } else {
            Self::String(value.to_owned())
        })
    }
}
