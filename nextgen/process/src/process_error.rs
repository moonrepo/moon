use moon_common::{Diagnostic, Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Failed to execute {} and capture output.", .bin.style(Style::Shell))]
    Capture {
        bin: String,
        #[source]
        error: std::io::Error,
    },

    #[error("Process {} failed with a {} exit code.", .bin.style(Style::Shell), .code.style(Style::Symbol))]
    ExitNonZero { bin: String, code: i32 },

    #[error("Process {} failed with a {} exit code.\n\n{}", .bin.style(Style::Shell), .code.style(Style::Symbol), .output.style(Style::MutedLight))]
    ExitNonZeroWithOutput {
        bin: String,
        code: i32,
        output: String,
    },

    #[error("Failed to execute {} and stream output.", .bin.style(Style::Shell))]
    Stream {
        bin: String,
        #[source]
        error: std::io::Error,
    },

    #[error("Failed to execute {} and stream and capture output.", .bin.style(Style::Shell))]
    StreamCapture {
        bin: String,
        #[source]
        error: std::io::Error,
    },

    #[error("Failed to write stdin to {}.", .bin.style(Style::Shell))]
    WriteInput {
        bin: String,
        #[source]
        error: std::io::Error,
    },
}

impl Diagnostic for ProcessError {}
