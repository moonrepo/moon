use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TokenExpanderError {
    #[diagnostic(code(token_expander::invalid_index))]
    #[error(
        "Token {} in task {} received an invalid type for index \"{index}\", must be a number.",
        .token.style(Style::Symbol),
        .target.style(Style::Label),
    )]
    InvalidTokenIndex {
        target: String,
        token: String,
        index: String,
    },

    #[diagnostic(code(token_expander::invalid_index_reference))]
    #[error(
        "Token {} in task {} is referencing another token or an invalid value. Only file paths or globs can be referenced by index.",
        .token.style(Style::Symbol),
        .target.style(Style::Label),
    )]
    InvalidTokenIndexReference { target: String, token: String },

    #[diagnostic(code(token_expander::invalid_scope))]
    #[error(
        "Token {} in task {} cannot be used within task {scope}.",
        .token.style(Style::Symbol),
        .target.style(Style::Label),
    )]
    InvalidTokenScope {
        target: String,
        token: String,
        scope: String,
    },

    #[diagnostic(code(token_expander::missing_in_index))]
    #[error(
        "Input index {index} does not exist for token {} in task {}.",
        .token.style(Style::Symbol),
        .target.style(Style::Label),
    )]
    MissingInIndex {
        index: usize,
        target: String,
        token: String,
    },

    #[diagnostic(code(token_expander::missing_out_index))]
    #[error(
        "Output index {index} does not exist for token {} in task {}.",
        .token.style(Style::Symbol),
        .target.style(Style::Label),
    )]
    MissingOutIndex {
        index: usize,
        target: String,
        token: String,
    },

    #[diagnostic(code(token_expander::unknown_file_group))]
    #[error(
        "Unknown file group {} used in token {}.",
        .group.style(Style::Id),
        .token.style(Style::Symbol),
    )]
    UnknownFileGroup { group: String, token: String },

    #[diagnostic(
        code(token_expander::unknown),
        url("https://moonrepo.dev/docs/concepts/token")
    )]
    #[error(
        "Unknown token {}.",
        .token.style(Style::Symbol),
    )]
    UnknownToken { token: String },
}
