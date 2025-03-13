use once_cell::sync::Lazy;
pub use regex::{Captures, Regex};

macro_rules! pattern {
    ($name:ident, $regex:literal) => {
        pub static $name: Lazy<regex::Regex> = Lazy::new(|| Regex::new($regex).unwrap());
    };
}

// Environment variables

pattern!(ENV_VAR, "\\$([A-Z0-9_]+)"); // $ENV_VAR
pattern!(ENV_VAR_DISTINCT, "^\\$([A-Z0-9_]+)$"); // $ENV_VAR
pattern!(ENV_VAR_GLOB_DISTINCT, "^\\$([A-Z0-9_*]+)$"); // $ENV_*

// Task tokens

pattern!(TOKEN_FUNC, "@([a-z]+)\\(([0-9A-Za-z_-]+)\\)");
pattern!(TOKEN_FUNC_DISTINCT, "^@([a-z]+)\\(([0-9A-Za-z_-]+)\\)$");
pattern!(
    TOKEN_VAR,
    "\\$(arch|language|osFamily|os|projectAlias|projectChannel|projectName|projectOwner|projectRoot|projectSource|projectStack|projectType|project|target|taskPlatform|taskToolchain|taskToolchains|taskType|task|timestamp|datetime|date|time|vcsBranch|vcsRepository|vcsRevision|workingDir|workspaceRoot)"
);
pattern!(
    TOKEN_VAR_DISTINCT,
    "^\\$(arch|language|osFamily|os|projectAlias|projectChannel|projectName|projectOwner|projectRoot|projectSource|projectStack|projectType|project|target|taskPlatform|taskToolchain|taskToolchains|taskType|task|timestamp|datetime|date|time|vcsBranch|vcsRepository|vcsRevision|workingDir|workspaceRoot)$"
);
