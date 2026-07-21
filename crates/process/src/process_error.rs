use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

impl ProcessError {
    /// Return the underlying process exit code, if this error represents a
    /// process that exited with a non-zero status (and not a signal).
    pub fn get_exit_code(&self) -> Option<i32> {
        match self {
            Self::ExitNonZero { code, .. } | Self::ExitNonZeroWithOutput { code, .. } => *code,
            _ => None,
        }
    }
}

#[derive(Error, Debug, Diagnostic)]
pub enum ProcessError {
    #[diagnostic(code(process::capture::failed))]
    #[error(
        "Failed to execute {} and capture output.",
        .bin.style(Style::Shell),
        // .error.to_string().style(Style::MutedLight),
    )]
    Capture {
        bin: String,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(process::failed))]
    #[error(
        "Process {} failed: {status}",
        .bin.style(Style::Shell),
    )]
    ExitNonZero {
        bin: String,
        status: String,
        code: Option<i32>,
    },

    #[diagnostic(code(process::failed))]
    #[error(
        "Process {} failed: {status} {}",
        .bin.style(Style::Shell),
        .output.style(Style::MutedLight),
    )]
    ExitNonZeroWithOutput {
        bin: String,
        status: String,
        code: Option<i32>,
        output: String,
    },

    #[diagnostic(code(process::stream::failed))]
    #[error(
        "Failed to execute {} and stream output.",
        .bin.style(Style::Shell),
        // .error.to_string().style(Style::MutedLight),
    )]
    Stream {
        bin: String,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(process::capture_stream::failed))]
    #[error(
        "Failed to execute {} and stream and capture output.",
        .bin.style(Style::Shell),
        // .error.to_string().style(Style::MutedLight),
    )]
    StreamCapture {
        bin: String,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(process::stdin::failed))]
    #[error(
        "Failed to write stdin to {}.",
        .bin.style(Style::Shell),
        // .error.to_string().style(Style::MutedLight),
    )]
    WriteInput {
        bin: String,
        #[source]
        error: Box<std::io::Error>,
    },
}
