use clap::ValueEnum;
use std::fmt;

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
                AppTheme::Dark => "dark",
                AppTheme::Light => "light",
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
                LogLevel::Off => "off",
                LogLevel::Error => "error",
                LogLevel::Warn => "warn",
                LogLevel::Info => "info",
                LogLevel::Debug => "debug",
                // Must map to tracing levels
                LogLevel::Trace | LogLevel::Verbose => "trace",
            }
        )?;

        Ok(())
    }
}
