use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum RunnerError {
    #[diagnostic(code(target_runner::missing_dep_hash))]
    #[error(
        "Encountered a missing hash for target {}, which is a dependency of {}.\nThis either means the dependency hasn't ran, has failed, or there's a misconfiguration.\n\nTry disabling the target's cache, or marking it as local.",
        .0.style(Style::Label),
        .1.style(Style::Label),
    )]
    MissingDependencyHash(String, String),

    #[diagnostic(code(target_runner::missing_output))]
    #[error("Target {} defines outputs, but none exist after being ran.", .0.style(Style::Label))]
    MissingOutput(String),
}
