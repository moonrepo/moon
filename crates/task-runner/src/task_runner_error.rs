use miette::Diagnostic;
use moon_common::{Style, Stylize};
use moon_process::ProcessError;
use moon_task::Target;
use std::path::PathBuf;
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

    #[diagnostic(code(task_runner::hash_check_failed))]
    #[error(
        "Task {} failed to run fingerprint check {}.",
        .target.style(Style::Label),
        .script.style(Style::Shell),
    )]
    FingerprintCheckFailed {
        target: Target,
        script: String,
        #[source]
        error: Box<ProcessError>,
    },

    #[diagnostic(code(task_runner::requirement_check_failed))]
    #[error(
        "Task {} is unable to run as the requirement check {} failed.",
        .target.style(Style::Label),
        .script.style(Style::Shell),
    )]
    RequirementCheckFailed {
        target: Target,
        script: String,
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

    #[diagnostic(code(task_runner::output::symlink_outside_workspace))]
    #[error(
        "Invalid task output, as the file {} is a symlink to {}, which exists outside of the workspace.",
        .output.style(Style::Path),
        .target.style(Style::Path),
    )]
    OutputSymlinkOutsideOfWorkspace { output: PathBuf, target: PathBuf },

    #[diagnostic(code(task_runner::output::file_outside_workspace))]
    #[error(
        "Invalid task output, as the file {} exists outside of the workspace.",
        .output.style(Style::Path),
    )]
    OutputFileOutsideOfWorkspace { output: PathBuf },

    #[diagnostic(code(task_runner::output::undeclared_output))]
    #[error(
        "Unable to hydrate cached output for task {}, as the file {} is not declared as an output.",
        .target.style(Style::Label),
        .output.style(Style::Path),
    )]
    OutputFileNotDeclared { target: Target, output: PathBuf },
}
