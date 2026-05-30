pub mod changed_files;
pub mod projects;
pub mod tasks;

use miette::IntoDiagnostic;
use tracing::debug;

pub(super) fn convert_to_regex(
    field: &str,
    value: &Option<String>,
) -> miette::Result<Option<regex::Regex>> {
    match value {
        Some(pattern) => {
            // case-insensitive by default
            let pattern = regex::Regex::new(&format!("(?i){pattern}")).into_diagnostic()?;

            debug!(
                "Filtering \"{}\" by matching against pattern \"{}\"",
                field, pattern
            );

            Ok(Some(pattern))
        }
        None => Ok(None),
    }
}
