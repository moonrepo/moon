use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("Exit with code")]
pub struct ExitCode(pub i32);
