use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TargetError {
    #[diagnostic(code(target::invalid_format))]
    #[error(
        "Invalid target {}, must be in the format of \"project:task\", with acceptable identifier characters.", .0.style(Style::Label)
    )]
    InvalidFormat(String),

    #[diagnostic(code(target::run_context::no_deps_scope))]
    #[error("Dependencies scope (^:) is not supported in run contexts.")]
    NoDepsInRunContext,

    #[diagnostic(code(target::run_context::no_self_scope))]
    #[error("Self scope (~:) is not supported in run contexts.")]
    NoSelfInRunContext,

    #[diagnostic(code(target::project_scope_required))]
    #[error(
        "Invalid target {}, requires fully-qualified project identifer (project:task).", .0.style(Style::Label)
    )]
    ProjectScopeRequired(String),

    #[diagnostic(code(target::task_scope_required))]
    #[error(
        "Invalid target {}, requires fully-qualified task identifer (project:task).", .0.style(Style::Label)
    )]
    TaskScopeRequired(String),

    #[diagnostic(code(target::tag_not_valid_for_default_project))]
    #[error(
        "Invalid target {}, tags are not supported for default project targets.", .0.style(Style::Label)
    )]
    TagNotValidForDefaultProject(String),

    #[diagnostic(code(target::missing_segments))]
    #[error("Target \":\" encountered. Wildcard project and task scopes are not supported.")]
    TooWild,
}
