mod builder;
mod errors;
mod parser;

use moon_error::MoonError;

pub use builder::*;
pub use errors::QueryError;
pub use parser::*;

pub trait Queryable {
    fn matches_criteria(&self, query: &Criteria) -> Result<bool, MoonError>;
}
