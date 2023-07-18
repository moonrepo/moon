use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum TokenExpanderError {
    #[diagnostic(code(project_graph::token::invalid_index))]
    #[error(
        "Token {} received an invalid type for index \"{index}\", must be a number.",
        .token.style(Style::Symbol)
    )]
    InvalidTokenIndex { token: String, index: String },

    #[diagnostic(code(project_graph::token::invalid_index_reference))]
    #[error(
        "Token {} is referencing another token or an invalid value. Only file paths or globs can be referenced by index.",
        .token.style(Style::Symbol)
    )]
    InvalidTokenIndexReference { token: String },

    #[diagnostic(code(project_graph::token::invalid_scope))]
    #[error("Token {} cannot be used within task {scope}.", .token.style(Style::Symbol))]
    InvalidTokenScope { token: String, scope: String },

    #[diagnostic(code(project_graph::token::missing_in_index))]
    #[error("Input index {index} does not exist for token {}.", .token.style(Style::Symbol))]
    MissingInIndex { index: usize, token: String },

    #[diagnostic(code(project_graph::token::missing_out_index))]
    #[error("Output index {index} does not exist for token {}.", .token.style(Style::Symbol))]
    MissingOutIndex { index: usize, token: String },

    #[diagnostic(code(project_graph::token::unknown_file_group))]
    #[error(
        "Unknown file group {} used in token {}.",
        .group.style(Style::Id),
        .token.style(Style::Symbol),
    )]
    UnknownFileGroup { group: String, token: String },

    #[diagnostic(
        code(project_graph::token::unknown),
        url("https://moonrepo.dev/docs/concepts/token")
    )]
    #[error(
        "Unknown token {}.",
        .token.style(Style::Symbol),
    )]
    UnknownToken { token: String },
}
