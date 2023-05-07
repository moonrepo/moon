mod builder;
mod errors;
mod parser;

pub use builder::*;
pub use errors::QueryError;
pub use parser::*;

pub trait Queryable {
    fn matches_criteria(&self, criteria: &Criteria) -> Result<bool, QueryError>;
}
