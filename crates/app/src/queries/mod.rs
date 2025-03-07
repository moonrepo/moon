pub mod hash;
pub mod hash_diff;
pub mod projects;
pub mod tasks;
pub mod touched_files;

use miette::IntoDiagnostic;
use tracing::trace;

pub(super) fn convert_to_regex(
    field: &str,
    value: &Option<String>,
) -> miette::Result<Option<regex::Regex>> {
    match value {
        Some(pattern) => {
            // case-insensitive by default
            let pattern = regex::Regex::new(&format!("(?i){pattern}")).into_diagnostic()?;

            trace!(
                "Filtering \"{}\" by matching against pattern \"{}\"",
                field, pattern
            );

            Ok(Some(pattern))
        }
        None => Ok(None),
    }
}
