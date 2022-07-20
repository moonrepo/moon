use lazy_static::lazy_static;
use moon_error::MoonError;

pub use regex::{Captures, Regex};

lazy_static! {
    // Capture group for IDs/names/etc
    static ref ID_GROUP: &'static str = "([A-Za-z]{1}[0-9A-Za-z_-]*)";
    static ref ID_CLEAN: Regex = Regex::new("[^a-z0-9_-]+").unwrap();

    pub static ref ID_PATTERN: Regex = Regex::new(&format!("^{}$", *ID_GROUP)).unwrap();
    pub static ref TARGET_PATTERN: Regex = Regex::new(
        "^(?P<project>(?:[A-Za-z]{1}[0-9A-Za-z_-]*|\\^|~))?:(?P<task>[A-Za-z]{1}[0-9A-Za-z_-]*)$").unwrap();

    // Token function: `@func(arg)`
    static ref TOKEN_GROUP: &'static str = "([0-9A-Za-z_-]+)";

    pub static ref TOKEN_FUNC_PATTERN: Regex = Regex::new(&format!("^@([a-z]+)\\({}\\)$", *TOKEN_GROUP)).unwrap();
    pub static ref TOKEN_FUNC_ANYWHERE_PATTERN: Regex = Regex::new(&format!("@([a-z]+)\\({}\\)", *TOKEN_GROUP)).unwrap();
    pub static ref TOKEN_VAR_PATTERN: Regex = Regex::new("\\$(language|projectRoot|projectSource|projectType|project|target|taskType|task|workspaceRoot)").unwrap();
}

pub fn create_regex(value: &str) -> Result<Regex, MoonError> {
    Regex::new(value).map_err(MoonError::Regex)
}

pub fn clean_id(id: &str) -> String {
    ID_CLEAN.replace(id, "").to_string()
}

pub fn matches_id(id: &str) -> bool {
    ID_PATTERN.is_match(id)
}

pub fn matches_target(target_id: &str) -> bool {
    TARGET_PATTERN.is_match(target_id)
}

pub fn matches_token_func(token: &str) -> bool {
    TOKEN_FUNC_PATTERN.is_match(token)
}

pub fn matches_token_var(token: &str) -> bool {
    TOKEN_VAR_PATTERN.is_match(token)
}
