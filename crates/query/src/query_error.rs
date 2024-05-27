use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum QueryError {
    #[diagnostic(code(query::empty_input))]
    #[error("Encountered an empty query. Did you forget to add criteria?")]
    EmptyInput,

    #[diagnostic(code(query::operator::logical_mismatch))]
    #[error("Cannot use both {} (&&) and {} (||) logical operators in the same group. Wrap in parentheses to create sub-groups.", "AND".style(Style::Symbol), "OR".style(Style::Symbol))]
    LogicalOperatorMismatch,

    #[diagnostic(code(query::unknown_field))]
    #[error("Unknown query field {}.", .0.style(Style::Id))]
    UnknownField(String),

    #[diagnostic(code(query::unknown_field_value))]
    #[error("Unknown query value {} for field {}.", .1.style(Style::Symbol), .0.style(Style::Id))]
    UnknownFieldValue(String, String),

    #[diagnostic(code(query::operator::unsupported))]
    #[error("Like operators (~ and !~) are not supported for field {}.", .0.style(Style::Id))]
    UnsupportedLikeOperator(String),

    #[diagnostic(code(query::parse::failed))]
    #[error("Failed to parse query:\n\n{}", .0.style(Style::MutedLight))]
    ParseFailure(String),
}
