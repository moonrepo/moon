use moon_action::{Action, ActionStatus, Operation};
use moon_app_context::AppContext;
use moon_cache_storage::Storage;
use moon_env_var::GlobalEnvBag;
use moon_hash::{ContentHasher, Digest};
use serde::Serialize;
use starbase_utils::fs::FileLock;

/// Holds the action's lock plus the hash manifest that records it as done.
///
/// The manifest is deliberately *not* stored up front: it is written only when
/// the caller reaches `persist_hash_manifest`, so an action that fails or is
/// killed leaves nothing behind claiming it succeeded. (A content-addressed
/// store has no delete, so the old write-then-remove-on-drop approach can't be
/// expressed against it anyway.)
pub struct HashLock {
    #[allow(dead_code)]
    lock: FileLock,
    digest: Digest,
    pending: Option<ContentHasher>,
}

impl HashLock {
    pub fn get_digest(&self) -> &Digest {
        &self.digest
    }

    pub async fn persist_hash_manifest(&mut self, storage: &Storage) -> miette::Result<()> {
        if let Some(hasher) = self.pending.take() {
            storage.store_hash_manifest_with_hasher(hasher).await?;
        }

        Ok(())
    }
}

pub fn create_hasher(
    action: &mut Action,
    _app_context: &AppContext,
    data: impl Serialize,
) -> miette::Result<ContentHasher> {
    let mut op = Operation::hash_generation();

    let mut hasher = ContentHasher::new(action.get_prefix());

    hasher.hash_content(data)?;

    let hash = hasher.generate_hash()?;

    op.meta.set_hash(&hash);
    op.finish(ActionStatus::Passed);

    action.operations.push(op);

    Ok(hasher)
}

pub async fn create_hash_and_return_lock(
    action: &mut Action,
    app_context: &AppContext,
    data: impl Serialize,
) -> miette::Result<HashLock> {
    let mut hasher = create_hasher(action, app_context, data)?;
    let digest = Digest::from_hasher(&mut hasher)?;

    let lock =
        app_context
            .cache_engine
            .create_lock(format!("{}-{}", action.get_prefix(), digest.hash))?;

    Ok(HashLock {
        lock,
        digest,
        pending: Some(hasher),
    })
}

pub async fn create_hash_and_return_lock_if_changed(
    action: &mut Action,
    app_context: &AppContext,
    fingerprint: impl Serialize,
    should_force: impl Fn() -> bool,
) -> miette::Result<Option<HashLock>> {
    let mut hasher = create_hasher(action, app_context, fingerprint)?;
    let digest = Digest::from_hasher(&mut hasher)?;

    let lock =
        app_context
            .cache_engine
            .create_lock(format!("{}-{}", action.get_prefix(), digest.hash))?;

    // If the hash manifest exists, then it has run before. Check this after
    // locking so that concurrent processes wait for in-progress actions.
    if !should_force()
        && app_context
            .cache_engine
            .storage
            .has_hash_manifest(&digest)
            .await
    {
        return Ok(None);
    }

    Ok(Some(HashLock {
        lock,
        digest,
        pending: Some(hasher),
    }))
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
