#![allow(unused_assignments)]

use miette::Diagnostic;
use moon_common::{Style, Stylize};
use moon_config::TaskDependencyType;
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
        "Task {} cannot depend on task {}, as the dependency cannot run in CI because {} is disabled{}. Because of this, the pipeline will not run tasks correctly. Instead you can either:\n\n- Enable {} for the dependency, so that both run in CI.\n- Set {} to {} for the dependency, so that the task runs in CI without it.\n- Disable {} for the task, so that neither runs in CI.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "options.runInCI".style(Style::Property),
        get_run_in_ci_default(.persistent),
        "options.runInCI".style(Style::Property),
        "options.runInCI".style(Style::Property),
        "skip".style(Style::Symbol),
        "options.runInCI".style(Style::Property),
    )]
    RunInCiDepRequirement {
        dep: Target,
        persistent: bool,
        task: Target,
    },

    #[diagnostic(code(task_builder::dependency::persistent_requirement))]
    #[error(
        "Non-persistent task {} cannot depend on persistent task {}.\nA task is marked persistent with the {} setting.\n\nIf you're looking to wait for the dependency to start running, mark the dependency with {} instead.\nIf you're looking to avoid the cache, disable {} instead.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "options.persistent".style(Style::Property),
        "type: 'wait'".style(Style::Property),
        "options.cache".style(Style::Property),
    )]
    PersistentDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_builder::dependency::persistent_cleanup_dep))]
    #[error(
        "Task {} cannot depend on persistent task {} as a {} dependency, as the dependency would never complete.\nA task is marked persistent with the {} setting.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "cleanup".style(Style::Symbol),
        "options.persistent".style(Style::Property),
    )]
    PersistentCleanupDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_builder::dependency::persistent_cleanup_task))]
    #[error(
        "Persistent task {} cannot depend on task {} as a {} dependency, as a persistent task never completes, and the dependency would run at the wrong time.\nA task is marked persistent with the {} setting.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "cleanup".style(Style::Symbol),
        "options.persistent".style(Style::Property),
    )]
    PersistentCleanupTaskRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_builder::dependency::interactive_wait_dep))]
    #[error(
        "Task {} cannot depend on interactive task {} as a {} dependency, as an interactive task runs in isolation, so nothing can run alongside it.\nA task is marked interactive with the {} setting.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        "wait".style(Style::Symbol),
        "options.interactive".style(Style::Property),
    )]
    InteractiveWaitDepRequirement { dep: Target, task: Target },

    #[diagnostic(code(task_builder::dependency::self_reference))]
    #[error(
        "Task {} cannot depend on itself as a {} dependency, as it can't run {} itself.",
        .task.style(Style::Label),
        .type_of.to_string().style(Style::Symbol),
        .relation,
    )]
    SelfDepRequirement {
        task: Target,
        type_of: TaskDependencyType,
        relation: &'static str,
    },

    #[diagnostic(code(task_builder::dependency::conflicting_types))]
    #[error(
        "Task {} depends on task {} with conflicting types, {} and {}. A dependency can only use one type.\n\nIf these dependencies are inherited, use the {} option, or the {} exclude/rename filters, to adjust them.",
        .task.style(Style::Label),
        .dep.style(Style::Label),
        .current_type.to_string().style(Style::Symbol),
        .other_type.to_string().style(Style::Symbol),
        "options.mergeDeps".style(Style::Property),
        "workspace.inheritedTasks".style(Style::Property),
    )]
    ConflictingDepType {
        dep: Target,
        task: Target,
        current_type: TaskDependencyType,
        other_type: TaskDependencyType,
    },

    #[diagnostic(
        code(task_builder::unknown_extends),
        help = "Has the task been renamed or excluded?"
    )]
    #[error(
        "Task {} is extending an unknown task {}.",
        .source_id.style(Style::Id),
        .target_id.style(Style::Id),
    )]
    UnknownExtendsSource {
        source_id: String,
        target_id: String,
    },

    #[diagnostic(code(task_builder::unknown_target))]
    #[error(
        "Invalid dependency {} for task {}, target does not exist.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
    )]
    UnknownDepTarget { dep: Target, task: Target },

    #[diagnostic(code(task_builder::unknown_target_in_project_deps))]
    #[error(
        "Invalid dependency {} for task {}, no matching targets in project dependencies. Mark the dependency as {} to allow no results.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
        "optional".style(Style::Property),
    )]
    UnknownDepTargetParentScope { dep: Target, task: Target },

    #[diagnostic(code(task_builder::unknown_target_in_tag))]
    #[error(
        "Invalid dependency {} for task {}, no matching targets within this tag. Mark the dependency as {} to allow no results.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
        "optional".style(Style::Property),
    )]
    UnknownDepTargetTagScope { dep: Target, task: Target },

    #[diagnostic(code(task_builder::unsupported_target_scope))]
    #[error(
        "Invalid dependency {} for task {}. All (:) scope is not supported.",
        .dep.style(Style::Label),
        .task.style(Style::Label),
    )]
    UnsupportedTargetScopeInDeps { dep: Target, task: Target },

    #[diagnostic(code(task_builder::unknown_project_input))]
    #[error(
        "Invalid project input {} for task {}. Only project dependencies of the parent project can be referenced as an input.",
        .dep.style(Style::Id),
        .task.style(Style::Label),
    )]
    UnknownProjectInput { dep: String, task: Target },

    #[diagnostic(code(task_builder::invalid_command_syntax))]
    #[error(
        "Failed to parse task {} with command {} at position {}. Either this command is too complex to parse, or we do not support this syntax. Instead you can either:\n\n- Use the {} setting, which supports raw shell syntax.\n- Rewrite as a list of strings instead of a single string.",
        .task.style(Style::Label),
        .command.style(Style::Shell),
        .position,
        "script".style(Style::Property),
    )]
    InvalidCommandSyntax {
        task: Target,
        command: String,
        position: String,
    },

    #[diagnostic(code(task_builder::unsupported_command_syntax))]
    #[error(
        "Unable to build task {}, as the {} and {} settings do not support pipes, redirects, multiple commands, or shell specific syntax. Instead you can either:\n\n- Use the {} setting, which supports raw shell syntax.\n- Wrap the command in a script file and execute that directly.",
        .task.style(Style::Label),
        "command".style(Style::Property),
        "args".style(Style::Property),
        "script".style(Style::Property),
    )]
    UnsupportedCommandSyntax { task: Target },
}

// Persistent tasks are disabled in CI by default, so the option
// that the error refers to may not have been configured at all
fn get_run_in_ci_default(persistent: &bool) -> &'static str {
    if *persistent {
        ", which is the default for persistent tasks"
    } else {
        ""
    }
}
