use thiserror::Error;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Cannot use both AND (&&) and OR (||) logical operators in the same group. Wrap in parentheses to create sub-groups.")]
    LogicalOperatorMismatch,

    #[error("Unknown query field \"{0}\".")]
    UnknownField(String),

    #[error("Unknown value \"{1}\" for field \"{0}\".")]
    UnknownFieldValue(String, String),

    #[error("Failed to parse query:\n{0}")]
    ParseFailure(String),
}
