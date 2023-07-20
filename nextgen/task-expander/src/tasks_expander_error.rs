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
        error: dotenvy::Error,
    },

    #[diagnostic(code(task_expander::persistent_requirement))]
    #[error(
        "Non-persistent task {} cannot depend on persistent task {}.\nA task is marked persistent with the {} or {} settings.\n\nIf you're looking to avoid the cache, disable {} instead.",
        .task.id.style(Style::Label),
        .dep.id.style(Style::Label),
        "local".style(Style::Symbol),
        "options.persistent".style(Style::Symbol),
        "options.cache".style(Style::Symbol),
    )]
    PersistentDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_expander::unsupported_target_scope))]
    #[error(
        "Invalid dependency {} for task {}. All (:) scope is not supported.",
        .dep.id.style(Style::Label),
        .task.id.style(Style::Label),
    )]
    UnsupportedTargetScopeInDeps { dep: Target, task: Target },
}
