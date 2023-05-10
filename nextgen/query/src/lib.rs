mod builder;
mod parser;
mod query_error;

pub use builder::*;
pub use parser::*;
pub use query_error::QueryError;

pub trait Queryable {
    fn matches_criteria(&self, criteria: &Criteria) -> Result<bool, QueryError>;
}
