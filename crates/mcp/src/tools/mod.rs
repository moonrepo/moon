pub mod action_tools;
pub mod project_tools;
pub mod task_tools;
pub mod vcs_tools;

use rust_mcp_sdk::schema::schema_utils::CallToolError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct ReportError(pub miette::Report);

impl Error for ReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn map_miette_error(report: miette::Report) -> CallToolError {
    CallToolError::new(ReportError(report))
}
