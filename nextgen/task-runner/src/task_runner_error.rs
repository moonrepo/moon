use miette::Diagnostic;
use moon_common::{Style, Stylize};
use moon_process::ProcessError;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TaskRunnerError {
    #[diagnostic(code(task_runner::run_failed))]
    #[error(
        "Task {} failed to run. View hash details with {}.",
        .target.style(Style::Label),
        .query.style(Style::Shell),
    )]
    RunFailed {
        target: String,
        query: String,
        #[source]
        error: ProcessError,
    },

    #[diagnostic(code(task_runner::missing_dependency_hash))]
    #[error(
        "Encountered a missing hash for target {}, which is a dependency of {}.\nThis either means the dependency hasn't ran, has failed, or there's a misconfiguration.\n\nTry disabling the target's cache, or marking it as local.",
        .dep_target.style(Style::Label),
        .target.style(Style::Label),
    )]
    MissingDependencyHash { dep_target: String, target: String },

    #[diagnostic(code(task_runner::missing_outputs))]
    #[error("Task {} defines outputs, but none exist after being ran.", .target.style(Style::Label))]
    MissingOutputs { target: String },
}
