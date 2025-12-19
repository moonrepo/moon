use miette::Diagnostic;
use moon_common::{Style, Stylize};
use moon_process::ProcessError;
use moon_task::Target;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TaskRunnerError {
    #[diagnostic(code(task_runner::run_failed))]
    #[error(
        "Task {} failed to run.",
        .target.style(Style::Label),
    )]
    RunFailed {
        target: Target,
        #[source]
        error: Box<ProcessError>,
    },

    #[diagnostic(code(task_runner::missing_dependency_hash))]
    #[error(
        "Encountered a missing hash for task {}, which is a dependency of {}.\nThis either means the dependency hasn't ran, has failed, or there's a misconfiguration.\n\nTry disabling the task's cache, or marking it as local.",
        .dep_target.style(Style::Label),
        .target.style(Style::Label),
    )]
    MissingDependencyHash { dep_target: Target, target: Target },

    #[diagnostic(code(task_runner::missing_outputs))]
    #[error(
        "Task {} defines outputs but after being ran, either none or not all of them exist.\nIf you require optional outputs, try using glob patterns instead.",
        .target.style(Style::Label)
    )]
    MissingOutputs { target: Target },
}
