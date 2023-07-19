use once_cell::sync::Lazy;
pub use regex::{Captures, Regex};

macro_rules! pattern {
    ($name:ident, $regex:literal) => {
        pub static $name: Lazy<regex::Regex> = Lazy::new(|| Regex::new($regex).unwrap());
    };
}

// Environment variables

pattern!(ENV_VAR, r"$([A-Z0-9_]+)");
pattern!(ENV_VAR_DISTINCT, r"^$([A-Z0-9_]+)$");
pattern!(ENV_VAR_SUBSTITUTE, r"$(?:{([A-Z0-9_]+)}|([A-Z0-9_]+))");
pattern!(ENV_VAR_SUBSTITUTE_STRICT, r"${([A-Z0-9_]+)}");

// Task tokens

pattern!(TOKEN_FUNC, "@([a-z]+)\\(([0-9A-Za-z_-]+)\\)");
pattern!(TOKEN_FUNC_DISTINCT, "^@([a-z]+)\\(([0-9A-Za-z_-]+)\\)$");
pattern!(
    TOKEN_VAR,
    r"$(language|projectAlias|projectRoot|projectSource|projectType|project|target|taskPlatform|taskType|task|workspaceRoot|timestamp|datetime|date|time)"
);
pattern!(
    TOKEN_VAR_DISTINCT,
    r"^$(language|projectAlias|projectRoot|projectSource|projectType|project|target|taskPlatform|taskType|task|workspaceRoot|timestamp|datetime|date|time)$"
);
