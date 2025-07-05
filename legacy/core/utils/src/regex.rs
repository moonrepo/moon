use miette::IntoDiagnostic;
use std::sync::LazyLock;

pub use regex::{Captures, Regex};

// Capture group for IDs/names/etc
pub static ID_GROUP: &str = "([A-Za-z]{1}[0-9A-Za-z/\\._-]*)";

pub static ID_CLEAN: LazyLock<regex::Regex> =
    LazyLock::new(|| create_regex("[^0-9A-Za-z/\\._-]+").unwrap());

pub static ID_PATTERN: LazyLock<regex::Regex> =
    LazyLock::new(|| create_regex(format!("^{ID_GROUP}$")).unwrap());

pub static TARGET_PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| {
    // Only target projects support `@` because of Node.js,
    // we don't want to support it in regular IDs!
    create_regex("^(?P<project>(?:[A-Za-z@#]{1}[0-9A-Za-z/\\._-]*|\\^|~))?:(?P<task>[A-Za-z]{1}[0-9A-Za-z/\\._-]*)$").unwrap()
});

// Input values
pub static ENV_VAR: LazyLock<regex::Regex> =
    LazyLock::new(|| create_regex("^\\$[A-Z0-9_]+$").unwrap());

pub static ENV_VAR_SUBSTITUTE: LazyLock<regex::Regex> =
    LazyLock::new(|| create_regex("\\$\\{([A-Z0-9_]+)\\}").unwrap());

#[inline]
pub fn create_regex<V: AsRef<str>>(value: V) -> miette::Result<Regex> {
    Regex::new(value.as_ref()).into_diagnostic()
}

#[inline]
pub fn clean_id(id: &str) -> String {
    ID_CLEAN.replace_all(id, "-").to_string()
}

#[inline]
pub fn matches_id(id: &str) -> bool {
    ID_PATTERN.is_match(id)
}

#[inline]
pub fn matches_target(target_id: &str) -> bool {
    TARGET_PATTERN.is_match(target_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod clean_ids {
        use super::*;

        #[test]
        fn doesnt_clean_supported_chars() {
            assert_eq!(clean_id("foo-bar_baz/123"), "foo-bar_baz/123");
        }

        #[test]
        fn replaces_unsupported_chars() {
            assert_eq!(clean_id("foo bar.baz$123"), "foo-bar.baz-123");
        }
    }
}
