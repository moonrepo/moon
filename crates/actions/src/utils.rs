use moon_action::{Action, ActionStatus, Operation};
use moon_app_context::AppContext;
use moon_env_var::GlobalEnvBag;
use serde::Serialize;
use starbase_utils::fs::FileLock;

pub async fn create_hash_and_return_lock_if_changed(
    action: &mut Action,
    app_context: &AppContext,
    data: impl Serialize,
) -> miette::Result<Option<FileLock>> {
    let cache_engine = &app_context.cache_engine;

    // Generate the hash and track the timings
    let mut hash_op = Operation::hash_generation();

    let mut hasher = cache_engine.hash.create_hasher(action.get_prefix());
    hasher.hash_content(data)?;

    let hash = hasher.generate_hash()?;

    hash_op.meta.set_hash(&hash);
    hash_op.finish(ActionStatus::Passed);

    action.operations.push(hash_op);

    // If the hash manifest exists, then it has ran before
    if cache_engine.hash.get_manifest_path(&hash).exists() {
        return Ok(None);
    }

    // Otherwise save the manifest and return a lock
    cache_engine.hash.save_manifest(&mut hasher)?;

    let lock = cache_engine.create_lock(format!("{}-{hash}", action.get_prefix()))?;

    Ok(Some(lock))
}

pub fn should_skip_action(key: &str) -> Option<String> {
    should_skip_action_matching(key, "true")
}

pub fn should_skip_action_matching<V: AsRef<str>>(key: &str, pattern: V) -> Option<String> {
    if let Some(value) = GlobalEnvBag::instance().get(key)
        && matches_pattern(&value, pattern.as_ref())
    {
        return Some(value);
    }

    None
}

fn matches_pattern(value: &str, pattern: &str) -> bool {
    if value.contains(',') {
        return value.split(',').any(|v| matches_pattern(v, pattern));
    }

    let pattern = pattern.to_lowercase();

    if value == "*"
        || value == "*:*"
        || value == "1"
        || value == "true"
        || value == pattern
        || pattern.is_empty()
    {
        return true;
    }

    if pattern.contains(':') {
        let mut left = pattern.split(':');
        let mut right = value.split(':');

        return match ((left.next(), left.next()), (right.next(), right.next())) {
            #[allow(clippy::nonminimal_bool)]
            ((Some(a1), Some(a2)), (Some(b1), Some(b2))) => {
                // foo:bar == foo:bar
                a1 == b1 && a2 == b2 ||
                // foo:bar == foo:*
                a1 == b1 && b2 == "*" ||
                // foo:bar == *:bar
                a2 == b2 && b1 == "*"
            }
            ((Some(a1), Some(_)), (Some(b1), None)) => {
                // foo:bar == foo
                a1 == b1
            }
            _ => false,
        };
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns() {
        assert!(matches_pattern("*", ""));
        assert!(matches_pattern("*:*", ""));
        assert!(matches_pattern("true", ""));

        assert!(matches_pattern("*", "node:20.0.0"));
        assert!(matches_pattern("node:*", "node:20.0.0"));
        assert!(matches_pattern("node", "node:20.0.0"));
        assert!(matches_pattern("node:20.0.0", "node:20.0.0"));
        assert!(!matches_pattern("rust", "node:20.0.0"));
        assert!(!matches_pattern("node:19.0.0", "node:20.0.0"));

        assert!(matches_pattern("foo,bar", "foo"));
        assert!(matches_pattern("foo,bar", "bar"));
        assert!(!matches_pattern("foo,bar", "baz"));
    }
}
