use crate::process_error::ProcessError;
use crate::shared_child::ChildExit;
use serde::{Deserialize, Serialize};
use std::process::ExitStatus;

pub use std::process::Output as NativeOutput;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Output {
    pub exit: ChildExit,
    pub stderr: Vec<u8>,
    pub stdout: Vec<u8>,
}

impl Output {
    pub fn code(&self) -> Option<i32> {
        self.status().and_then(|status| status.code())
    }

    pub fn status(&self) -> Option<ExitStatus> {
        match self.exit {
            ChildExit::Completed(status) => Some(status),
            _ => None,
        }
    }

    pub fn success(&self) -> bool {
        self.status().is_some_and(|status| status.success())
    }

    pub fn to_info(&self) -> OutputInfo {
        OutputInfo {
            exit_code: self.code(),
            signal: match self.exit {
                ChildExit::Completed(_) => None,
                ChildExit::Interrupted => Some(2),
                ChildExit::Killed => Some(9),
                ChildExit::Terminated => Some(15),
            },
            stderr: if self.stderr.is_empty() {
                None
            } else {
                Some(output_to_trimmed_string(&self.stderr))
            },
            stdout: if self.stdout.is_empty() {
                None
            } else {
                Some(output_to_trimmed_string(&self.stdout))
            },
        }
    }

    pub fn to_error(&self, bin: impl AsRef<str>, with_message: bool) -> ProcessError {
        let bin = bin.as_ref().to_owned();
        let code = self.code();

        let status = match &self.exit {
            ChildExit::Completed(status) => match status.code() {
                Some(code) => format!("exit code {code}"),
                None => status.to_string(),
            },
            ChildExit::Interrupted => "interrupted".into(),
            ChildExit::Killed => "killed".into(),
            ChildExit::Terminated => "terminated".into(),
        };

        if !with_message {
            return ProcessError::ExitNonZero { bin, status, code };
        }

        let mut message = output_to_trimmed_string(&self.stderr);

        if message.is_empty() {
            message = output_to_trimmed_string(&self.stdout);
        }

        // Make error message nicer to look at
        if !message.is_empty() {
            message = format!("\n\n{message}");
        }

        ProcessError::ExitNonZeroWithOutput {
            bin,
            status,
            code,
            output: message,
        }
    }
}

#[inline]
pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8_lossy(data).into_owned()
}

#[inline]
pub fn output_to_trimmed_string(data: &[u8]) -> String {
    output_to_string(data).trim().to_owned()
}
