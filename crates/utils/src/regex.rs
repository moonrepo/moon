use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Capture group for IDs/names/etc
    static ref ID_GROUP: &'static str = "([A-Za-z]{1}[0-9A-Za-z_-]*)";

    pub static ref ID_PATTERN: Regex = Regex::new(&format!("^{}$", ID_GROUP.to_string())).unwrap();
    pub static ref TARGET_PATTERN: Regex = Regex::new(&format!("^{}:{}$", ID_GROUP.to_string(), ID_GROUP.to_string())).unwrap();

    // Token function: `@func(arg)`
    static ref TOKEN_GROUP: &'static str = "([0-9A-Za-z_-]+)";

    pub static ref TOKEN_FUNC_PATTERN: Regex = Regex::new(&format!("^@([a-z]+)\\({}\\)$", TOKEN_GROUP.to_string())).unwrap();
    pub static ref TOKEN_FUNC_ANYWHERE_PATTERN: Regex = Regex::new(&format!("@([a-z]+)\\({}\\)", TOKEN_GROUP.to_string())).unwrap();
}

pub fn matches_id(id: &str) -> bool {
    ID_PATTERN.is_match(id)
}

pub fn matches_target(target: &str) -> bool {
    TARGET_PATTERN.is_match(target)
}
