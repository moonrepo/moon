use moon_action::Action;
use moon_actions::utils::create_hash_and_return_lock_if_changed;
use moon_app_context::AppContext;
use moon_hash::{ContentHasher, Digest};
use moon_test_utils::WorkspaceMocker;
use serde::Serialize;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Clone, Serialize)]
struct TestFingerprint {
    input: &'static str,
}

fn create_workspace() -> (Sandbox, Arc<AppContext>) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

    (sandbox, Arc::new(mocker.mock_app_context()))
}

/// The digest an action's fingerprint hashes to, so tests can ask storage
/// whether the manifest that marks it done was written.
fn fingerprint_digest(action: &Action, fingerprint: &TestFingerprint) -> Digest {
    let mut hasher = ContentHasher::new(action.get_prefix());
    hasher.hash_content(fingerprint).unwrap();

    Digest::from_hasher(&mut hasher).unwrap()
}

async fn has_hash_manifest(
    app_context: &AppContext,
    action: &Action,
    fingerprint: &TestFingerprint,
) -> bool {
    app_context
        .cache_engine
        .storage
        .has_hash_manifest(&fingerprint_digest(action, fingerprint))
        .await
}

fn has_vendor_contents(vendor_dir: &Path) -> bool {
    vendor_dir.exists()
        && fs::read_dir(vendor_dir).is_ok_and(|mut contents| contents.next().is_some())
}

mod hash_locks {
    use super::*;

    #[tokio::test]
    async fn stores_no_hash_manifest_when_dropped_without_persisting() {
        // The manifest marks the action as done, so it must only be written once
        // the action actually succeeded. A lock dropped without persisting (a
        // failed or killed action) must leave nothing behind, or the next run
        // would skip work that never happened.
        let (_, app_context) = create_workspace();
        let fingerprint = TestFingerprint { input: "failure" };
        let mut action = Action::default();

        let lock = create_hash_and_return_lock_if_changed(
            &mut action,
            &app_context,
            fingerprint.clone(),
            || false,
        )
        .await
        .unwrap()
        .unwrap();

        assert!(!has_hash_manifest(&app_context, &action, &fingerprint).await);

        drop(lock);

        assert!(!has_hash_manifest(&app_context, &action, &fingerprint).await);
    }

    #[tokio::test]
    async fn stores_the_hash_manifest_on_persist_and_then_short_circuits() {
        let (_, app_context) = create_workspace();
        let fingerprint = TestFingerprint { input: "success" };
        let mut action = Action::default();

        let mut lock = create_hash_and_return_lock_if_changed(
            &mut action,
            &app_context,
            fingerprint.clone(),
            || false,
        )
        .await
        .unwrap()
        .unwrap();

        lock.persist_hash_manifest(&app_context.cache_engine.storage)
            .await
            .unwrap();
        app_context
            .cache_engine
            .storage
            .wait_for_background_tasks()
            .await
            .unwrap();
        drop(lock);

        assert!(has_hash_manifest(&app_context, &action, &fingerprint).await);

        // Having run once, the same fingerprint is now a no-op.
        assert!(
            create_hash_and_return_lock_if_changed(&mut action, &app_context, fingerprint, || {
                false
            })
            .await
            .unwrap()
            .is_none()
        );
    }

    #[tokio::test]
    async fn does_not_save_hash_manifest_when_lock_creation_fails() {
        let (_, app_context) = create_workspace();
        let fingerprint = TestFingerprint {
            input: "lock-failure",
        };
        let mut action = Action::default();
        let mut hasher = ContentHasher::new(action.get_prefix());

        hasher.hash_content(fingerprint.clone()).unwrap();

        let hash = hasher.generate_hash().unwrap();
        let lock_path = app_context
            .cache_engine
            .cache_dir
            .join("locks")
            .join(format!("unknown-{hash}.lock"));

        fs::create_dir_all(lock_path).unwrap();

        assert!(
            create_hash_and_return_lock_if_changed(
                &mut action,
                &app_context,
                fingerprint.clone(),
                || false
            )
            .await
            .is_err()
        );
        assert!(!has_hash_manifest(&app_context, &action, &fingerprint).await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn revalidates_forced_vendor_installs_after_waiting_on_lock() {
        let (sandbox, app_context) = create_workspace();
        let fingerprint = TestFingerprint { input: "vendor" };
        let vendor_dir = sandbox.path().join("vendor");
        let mut action = Action::default();

        let mut lock = create_hash_and_return_lock_if_changed(
            &mut action,
            &app_context,
            fingerprint.clone(),
            || true,
        )
        .await
        .unwrap()
        .unwrap();

        let (checked_tx, checked_rx) = mpsc::channel();
        let app_context_for_task = Arc::clone(&app_context);
        let vendor_dir_for_task = vendor_dir.clone();
        let fingerprint_for_task = fingerprint.clone();

        let handle = tokio::task::spawn_blocking(move || {
            let mut action = Action::default();

            tokio::runtime::Handle::current().block_on(async {
                create_hash_and_return_lock_if_changed(
                    &mut action,
                    &app_context_for_task,
                    fingerprint_for_task,
                    || {
                        let _ = checked_tx.send(());

                        !has_vendor_contents(&vendor_dir_for_task)
                    },
                )
                .await
                .unwrap()
                .is_none()
            })
        });

        assert!(checked_rx.recv_timeout(Duration::from_millis(100)).is_err());

        sandbox.create_file("vendor/dependency", "installed");
        lock.persist_hash_manifest(&app_context.cache_engine.storage)
            .await
            .unwrap();
        app_context
            .cache_engine
            .storage
            .wait_for_background_tasks()
            .await
            .unwrap();
        drop(lock);

        assert!(handle.await.unwrap());
        assert!(checked_rx.recv_timeout(Duration::from_secs(1)).is_ok());
        assert!(has_hash_manifest(&app_context, &action, &fingerprint).await);
    }
}
