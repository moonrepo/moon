pub use regex::{Captures, Regex};
use rustc_hash::FxHashMap;
use schematic::{MergeError, MergeResult, PartialConfig};
use std::hash::Hash;
use std::sync::LazyLock;

macro_rules! pattern {
    ($name:ident, $regex:literal) => {
        pub static $name: LazyLock<regex::Regex> = LazyLock::new(|| Regex::new($regex).unwrap());
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
    "\\$(arch|language|osFamily|os|projectAlias|projectAliases|projectChannel|projectName|projectLayer|projectOwner|projectRoot|projectSource|projectStack|project|target|taskToolchain|taskToolchains|taskType|task|timestamp|datetime|date|time|vcsBranch|vcsRepository|vcsRevision|workingDir|workspaceRoot)"
);
pattern!(
    TOKEN_VAR_DISTINCT,
    "^\\$(arch|language|osFamily|os|projectAlias|projectAliases|projectChannel|projectName|projectLayer|projectOwner|projectRoot|projectSource|projectStack|project|target|taskToolchain|taskToolchains|taskType|task|timestamp|datetime|date|time|vcsBranch|vcsRepository|vcsRevision|workingDir|workspaceRoot)$"
);

pub fn merge_iter<I, V, C>(mut prev: I, next: I, _: &C) -> MergeResult<I>
where
    I: Extend<V> + IntoIterator<Item = V>,
{
    prev.extend(next);
    Ok(Some(prev))
}

pub fn merge_plugin_partials<K, V>(
    mut prev: FxHashMap<K, V>,
    next: FxHashMap<K, V>,
    context: &V::Context,
) -> MergeResult<FxHashMap<K, V>>
where
    K: Eq + Hash,
    V: PartialConfig,
{
    for (key, value) in next {
        match prev.get_mut(&key) {
            Some(existing) => {
                existing.merge(context, value).map_err(MergeError::new)?;
            }
            None => {
                prev.insert(key, value);
            }
        }
    }

    Ok(Some(prev))
}
