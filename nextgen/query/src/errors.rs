use moon_common::{Diagnostic, Style, Stylize};
use starbase_utils::glob::GlobError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Encountered an empty query. Did you forget to add criteria?")]
    EmptyInput,

    #[error("Cannot use both AND (&&) and OR (||) logical operators in the same group. Wrap in parentheses to create sub-groups.")]
    LogicalOperatorMismatch,

    #[error("Unknown query field {}.", .0.style(Style::Id))]
    UnknownField(String),

    #[error("Unknown query value {} for field {}.", .1.style(Style::Symbol), .0.style(Style::Id))]
    UnknownFieldValue(String, String),

    #[error("Like operators (~ and !~) are not supported for field {}.", .0.style(Style::Id))]
    UnsupportedLikeOperator(String),

    #[error("Failed to parse query:\n\n{}", .0.style(Style::MutedLight))]
    ParseFailure(String),

    #[error(transparent)]
    Glob(#[from] GlobError),
}

impl Diagnostic for QueryError {}
