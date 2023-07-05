use miette::Diagnostic;
use moon_target::Target;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectGraphError {
    #[diagnostic(code(project_graph::task::overlapping_outputs))]
    #[error(
        "Tasks {} have configured the same output {}. Overlapping outputs is not supported as it can cause non-deterministic results.",
        .targets.iter().map(|t| t.id.style(Style::Label)).collect::<Vec<_>>().join(", "),
        .output.style(Style::File),
    )]
    OverlappingTaskOutputs {
        output: String,
        targets: Vec<Target>,
    },

    #[diagnostic(code(project_graph::task_dep::persistent_requirement))]
    #[error(
        "Non-persistent task {} cannot depend on persistent task {}.\nA task is marked persistent with the {} or {} settings.\n\nIf you're looking to avoid the cache, disable {} instead.",
        .0.style(Style::Label),
        .1.style(Style::Label),
        "local".style(Style::Symbol),
        "options.persistent".style(Style::Symbol),
        "options.cache".style(Style::Symbol),
    )]
    PersistentDepRequirement(String, String),

    #[diagnostic(code(project_graph::task_dep::unsupported_target_scope))]
    #[error(
        "Invalid dependency {} for task {}. All (:) and tag (#tag:) scopes are not supported.",
        .0.style(Style::Label),
        .1.style(Style::Label),
    )]
    UnsupportedTargetScopeInDeps(String, String),
}

#[derive(Error, Debug, Diagnostic)]
pub enum TokenError {
    #[error(
        "Token {} received an invalid type for index \"{1}\", must be a number.", .0.style(Style::Symbol)
    )]
    InvalidIndexType(String, String), // token, index

    #[error("Input index {1} doesn't exist for token {}.", .0.style(Style::Symbol))]
    InvalidInIndex(String, u8), // token, index

    #[error("Output index {1} doesn't exist for token {}.", .0.style(Style::Symbol))]
    InvalidOutIndex(String, u8), // token, index

    #[error("Output token {} may not reference outputs using token functions.", .0.style(Style::Symbol))]
    InvalidOutNoTokenFunctions(String),

    #[error("Token {} cannot be used within {}.", .0.style(Style::Symbol), .1.style(Style::Symbol))]
    InvalidTokenContext(String, String), // token, context

    #[error("Unknown file group {} used in token {}.", .1.style(Style::Id), .0.style(Style::Symbol))]
    UnknownFileGroup(String, String), // token, file group

    #[error("Unknown token function {}.", .0.style(Style::Symbol))]
    UnknownTokenFunc(String), // token
}
