use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use moon_task::Target;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TasksBuilderError {
    #[diagnostic(code(task_builder::dependency::no_allowed_failures))]
    #[error(
        "Task {} cannot depend on task {}, as it is allowed to fail, which may cause unwanted side-effects.\nA task is marked to allow failure with the {} setting.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "options.allowFailure".style(Style::Property),
    )]
    AllowFailureDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_builder::dependency::run_in_ci_mismatch))]
    #[error(
        "Task {} cannot depend on task {}, as the dependency cannot run in CI because {} is disabled. Because of this, the pipeline will not run tasks correctly.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "options.runInCI".style(Style::Property),
    )]
    RunInCiDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_builder::dependency::persistent_requirement))]
    #[error(
        "Non-persistent task {} cannot depend on persistent task {}.\nA task is marked persistent with the {} setting.\n\nIf you're looking to avoid the cache, disable {} instead.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "options.persistent".style(Style::Property),
        "options.cache".style(Style::Property),
    )]
    PersistentDepRequirement { dep: Target, task: Target },

    #[diagnostic(
        code(task_builder::unknown_extends),
        help = "Has the task been renamed or excluded?"
    )]
    #[error(
        "Task {} is extending an unknown task {}.",
        .source_id.style(Style::Id),
        .target_id.style(Style::Id),
    )]
    UnknownExtendsSource { source_id: Id, target_id: Id },

    #[diagnostic(code(task_builder::unknown_target))]
    #[error(
        "Invalid dependency {} for {}, target does not exist.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
    )]
    UnknownDepTarget { dep: Target, task: Target },

    #[diagnostic(code(task_builder::unknown_target_in_project_deps))]
    #[error(
        "Invalid dependency {} for {}, no matching targets in project dependencies. Mark the dependency as {} to allow no results.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
        "optional".style(Style::Property),
    )]
    UnknownDepTargetParentScope { dep: Target, task: Target },

    #[diagnostic(code(task_builder::unknown_target_in_tag))]
    #[error(
        "Invalid dependency {} for {}, no matching targets within this tag. Mark the dependency as {} to allow no results.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
        "optional".style(Style::Property),
    )]
    UnknownDepTargetTagScope { dep: Target, task: Target },

    #[diagnostic(code(task_builder::unsupported_target_scope))]
    #[error(
        "Invalid dependency {} for {}. All (:) scope is not supported.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
    )]
    UnsupportedTargetScopeInDeps { dep: Target, task: Target },
}
