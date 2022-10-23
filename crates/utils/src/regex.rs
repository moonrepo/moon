use lazy_static::lazy_static;
use moon_error::MoonError;

pub use regex::{Captures, Regex};

lazy_static! {
    // Capture group for IDs/names/etc
    static ref ID_GROUP: &'static str = "([A-Za-z]{1}[0-9A-Za-z/_-]*)";
    static ref ID_CLEAN: Regex = Regex::new("[^0-9A-Za-z/_-]+").unwrap();

    pub static ref ID_PATTERN: Regex = Regex::new(&format!("^{}$", *ID_GROUP)).unwrap();
    pub static ref TARGET_PATTERN: Regex = Regex::new(
        // Only target projects support `@` because of Node.js,
        // we don't want to support it in regular IDs!
        "^(?P<project>(?:[A-Za-z@]{1}[0-9A-Za-z/_-]*|\\^|~))?:(?P<task>[A-Za-z]{1}[0-9A-Za-z/_-]*)$").unwrap();

    // Input values
    pub static ref ENV_VAR: Regex = Regex::new("^\\$[A-Z0-9_]+$").unwrap();

    // Token function: `@func(arg)`
    static ref TOKEN_GROUP: &'static str = "([0-9A-Za-z_-]+)";

    pub static ref TOKEN_FUNC_PATTERN: Regex = Regex::new(&format!("^@([a-z]+)\\({}\\)$", *TOKEN_GROUP)).unwrap();
    pub static ref TOKEN_FUNC_ANYWHERE_PATTERN: Regex = Regex::new(&format!("@([a-z]+)\\({}\\)", *TOKEN_GROUP)).unwrap();
    pub static ref TOKEN_VAR_PATTERN: Regex = Regex::new("\\$(language|projectRoot|projectSource|projectType|project|target|taskPlatform|taskType|task|workspaceRoot)").unwrap();

    // Task commands (these are not exhaustive)
    pub static ref NODE_COMMAND: regex::Regex =
                Regex::new("^(node|nodejs|npm|npx|yarn|pnpm|corepack)$").unwrap();

    pub static ref UNIX_SYSTEM_COMMAND: regex::Regex =
                Regex::new("^(bash|cat|cd|chmod|cp|docker|echo|find|git|grep|make|mkdir|mv|pwd|rm|rsync|svn)$").unwrap();

    pub static ref WINDOWS_SYSTEM_COMMAND: regex::Regex =
                Regex::new("^(cd|cmd|copy|del|dir|echo|erase|find|git|mkdir|move|rd|rename|replace|rmdir|svn|xcopy)$").unwrap();
}

#[inline]
pub fn create_regex(value: &str) -> Result<Regex, MoonError> {
    Regex::new(value).map_err(MoonError::Regex)
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
            assert_eq!(clean_id("foo bar.baz$123"), "foo-bar-baz-123");
        }
    }
}
