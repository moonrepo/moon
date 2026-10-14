pub use regex::{Captures, Regex};
use std::sync::LazyLock;

macro_rules! pattern {
    ($name:ident, $regex:literal) => {
        pub static $name: LazyLock<regex::Regex> = LazyLock::new(|| Regex::new($regex).unwrap());
    };
}

// Environment variables

pattern!(ENV_VAR_DISTINCT, "^\\$([A-Z0-9_]+)$"); // $ENV_VAR
pattern!(ENV_VAR_GLOB_DISTINCT, "^\\$([A-Z0-9_*]+)$"); // $ENV_*

// Task tokens

pattern!(TOKEN_FUNC, "@([a-z]+)\\(([0-9A-Za-z_-]+)\\)");
pattern!(TOKEN_FUNC_DISTINCT, "^@([a-z]+)\\(([0-9A-Za-z_-]+)\\)$");
pattern!(
    TOKEN_VAR,
    "\\$(arch|language|osFamily|os|projectAliases|projectAlias|projectChannel|projectId|projectLayer|projectName|projectTitle|projectOwner|projectRoot|projectSource|projectStack|project|target|taskId|taskToolchains|taskToolchain|taskType|task|timestamp|datetime|date|time|vcsBranch|vcsRepository|vcsRevision|workingDir|workspaceRoot)"
);
pattern!(
    TOKEN_VAR_DISTINCT,
    "^\\$(arch|language|osFamily|os|projectAliases|projectAlias|projectChannel|projectId|projectLayer|projectName|projectTitle|projectOwner|projectRoot|projectSource|projectStack|project|target|taskId|taskToolchains|taskToolchain|taskType|task|timestamp|datetime|date|time|vcsBranch|vcsRepository|vcsRevision|workingDir|workspaceRoot)$"
);
