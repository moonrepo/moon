use miette::Diagnostic;
use moon_common::{Style, Stylize};
use moon_task::Target;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TasksExpanderError {
    #[diagnostic(code(task_expander::invalid_env_file))]
    #[error("Failed to parse env file {}.", .path.style(Style::Path))]
    InvalidEnvFile {
        path: PathBuf,
        #[source]
        error: Box<dotenvy::Error>,
    },

    #[diagnostic(code(task_expander::dependency::no_allowed_failures))]
    #[error(
        "Task {} cannot depend on task {}, as it is allowed to fail, which may cause unwanted side-effects.\nA task is marked to allow failure with the {} setting.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "options.allowFailure".style(Style::Symbol),
    )]
    AllowFailureDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_expander::dependency::persistent_requirement))]
    #[error(
        "Non-persistent task {} cannot depend on persistent task {}.\nA task is marked persistent with the {} or {} settings.\n\nIf you're looking to avoid the cache, disable {} instead.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "local".style(Style::Symbol),
        "options.persistent".style(Style::Symbol),
        "options.cache".style(Style::Symbol),
    )]
    PersistentDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_expander::dependency::run_in_ci_mismatch))]
    #[error(
        "Task {} cannot depend on task {}, as the dependency cannot run in CI because {} is disabled. Because of this, the pipeline will not run tasks correctly.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "options.runInCI".style(Style::Symbol),
    )]
    RunInCiDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_expander::unknown_target))]
    #[error(
        "Invalid dependency {} for {}, target does not exist.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
    )]
    UnknownTarget { dep: Target, task: Target },

    #[diagnostic(code(task_expander::unsupported_target_scope))]
    #[error(
        "Invalid dependency {} for {}. All (:) scope is not supported.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
    )]
    UnsupportedTargetScopeInDeps { dep: Target, task: Target },
}
