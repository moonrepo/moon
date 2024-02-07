use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("Exit with code")]
pub struct ExitCode(pub i32);

#[derive(Debug, Diagnostic, Error)]
pub enum AppError {
    #[diagnostic(code(app::ci::no_shallow))]
    #[error(
        "CI requires a full VCS history to operate correctly. Please avoid shallow checkouts."
    )]
    CiNoShallowHistory,
}
