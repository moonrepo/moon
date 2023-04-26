use moon_error::MoonError;
use once_cell::sync::Lazy;

pub use regex::{Captures, Regex};

// Capture group for IDs/names/etc
pub static ID_GROUP: &str = "([A-Za-z]{1}[0-9A-Za-z/\\._-]*)";

pub static ID_CLEAN: Lazy<regex::Regex> =
    Lazy::new(|| create_regex("[^0-9A-Za-z/\\._-]+").unwrap());

pub static ID_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| create_regex(format!("^{}$", ID_GROUP)).unwrap());

pub static TARGET_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    // Only target projects support `@` because of Node.js,
    // we don't want to support it in regular IDs!
    create_regex("^(?P<project>(?:[A-Za-z@]{1}[0-9A-Za-z/\\._-]*|\\^|~))?:(?P<task>[A-Za-z]{1}[0-9A-Za-z/\\._-]*)$").unwrap()
});

// Input values
pub static ENV_VAR: Lazy<regex::Regex> = Lazy::new(|| create_regex("^\\$[A-Z0-9_]+$").unwrap());

pub static ENV_VAR_SUBSTITUTE: Lazy<regex::Regex> =
    Lazy::new(|| create_regex("\\$\\{([A-Z0-9_]+)\\}").unwrap());

// Token function: `@func(arg)`
pub static TOKEN_GROUP: &str = "([0-9A-Za-z_-]+)";

pub static TOKEN_FUNC_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| create_regex(format!("^@([a-z]+)\\({}\\)$", TOKEN_GROUP)).unwrap());

pub static TOKEN_FUNC_ANYWHERE_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| create_regex(format!("@([a-z]+)\\({}\\)", TOKEN_GROUP)).unwrap());

pub static TOKEN_VAR_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    create_regex("\\$(language|projectAlias|projectRoot|projectSource|projectType|project|target|taskPlatform|taskType|task|workspaceRoot|timestamp|datetime|date|time)").unwrap()
});

// Task commands (these are not exhaustive)
pub static UNIX_SYSTEM_COMMAND: Lazy<regex::Regex> = Lazy::new(|| {
    create_regex(
        "^(bash|cat|cd|chmod|cp|docker|echo|find|git|grep|make|mkdir|mv|pwd|rm|rsync|svn)$",
    )
    .unwrap()
});

pub static WINDOWS_SYSTEM_COMMAND: Lazy<regex::Regex> = Lazy::new(|| {
    create_regex(
        "^(cd|cmd|copy|del|dir|echo|erase|find|git|mkdir|move|rd|rename|replace|rmdir|svn|xcopy)$",
    )
    .unwrap()
});

#[inline]
pub fn create_regex<V: AsRef<str>>(value: V) -> Result<Regex, MoonError> {
    Regex::new(value.as_ref()).map_err(MoonError::Regex)
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

#[inline]
pub fn matches_token_func(token: &str) -> bool {
    TOKEN_FUNC_PATTERN.is_match(token)
}

#[inline]
pub fn matches_token_var(token: &str) -> bool {
    TOKEN_VAR_PATTERN.is_match(token)
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
