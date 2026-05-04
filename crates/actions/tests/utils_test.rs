use moon_action::Action;
use moon_actions::utils::create_hash_and_return_lock_if_changed;
use moon_app_context::AppContext;
use moon_test_utils2::WorkspaceMocker;
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

fn count_hash_manifests(app_context: &AppContext) -> usize {
    fs::read_dir(&app_context.cache_engine.hash.hashes_dir)
        .map(|entries| entries.count())
        .unwrap_or_default()
}

fn has_vendor_contents(vendor_dir: &Path) -> bool {
    vendor_dir.exists()
        && fs::read_dir(vendor_dir).is_ok_and(|mut contents| contents.next().is_some())
}

mod hash_locks {
    use super::*;

    #[test]
    fn removes_hash_manifest_when_dropped_without_persisting() {
        let (_, app_context) = create_workspace();
        let fingerprint = TestFingerprint { input: "failure" };
        let mut action = Action::default();

        let lock = create_hash_and_return_lock_if_changed(
            &mut action,
            &app_context,
            fingerprint.clone(),
            || false,
        )
        .unwrap()
        .unwrap();

        assert_eq!(count_hash_manifests(&app_context), 1);

        drop(lock);

        assert_eq!(count_hash_manifests(&app_context), 0);

        let mut lock = create_hash_and_return_lock_if_changed(
            &mut action,
            &app_context,
            fingerprint.clone(),
            || false,
        )
        .unwrap()
        .unwrap();

        lock.persist_hash_manifest();
        drop(lock);

        assert_eq!(count_hash_manifests(&app_context), 1);

        assert!(
            create_hash_and_return_lock_if_changed(&mut action, &app_context, fingerprint, || {
                false
            })
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn does_not_save_hash_manifest_when_lock_creation_fails() {
        let (_, app_context) = create_workspace();
        let fingerprint = TestFingerprint {
            input: "lock-failure",
        };
        let mut action = Action::default();
        let mut hasher = app_context
            .cache_engine
            .hash
            .create_hasher(action.get_prefix());

        hasher.hash_content(fingerprint.clone()).unwrap();

        let hash = hasher.generate_hash().unwrap();
        let lock_path = app_context
            .cache_engine
            .cache_dir
            .join("locks")
            .join(format!("unknown-{hash}.lock"));

        fs::create_dir_all(lock_path).unwrap();

        assert!(
            create_hash_and_return_lock_if_changed(&mut action, &app_context, fingerprint, || {
                false
            })
            .is_err()
        );
        assert_eq!(count_hash_manifests(&app_context), 0);
    }

    #[test]
    fn revalidates_forced_vendor_installs_after_waiting_on_lock() {
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
        .unwrap()
        .unwrap();

        let (checked_tx, checked_rx) = mpsc::channel();
        let app_context_for_thread = Arc::clone(&app_context);
        let vendor_dir_for_thread = vendor_dir.clone();

        let handle = std::thread::spawn(move || {
            let mut action = Action::default();

            create_hash_and_return_lock_if_changed(
                &mut action,
                &app_context_for_thread,
                fingerprint,
                || {
                    let _ = checked_tx.send(());

                    !has_vendor_contents(&vendor_dir_for_thread)
                },
            )
            .unwrap()
            .is_none()
        });

        assert!(checked_rx.recv_timeout(Duration::from_millis(100)).is_err());

        sandbox.create_file("vendor/dependency", "installed");
        lock.persist_hash_manifest();
        drop(lock);

        assert!(handle.join().unwrap());
        assert!(checked_rx.recv_timeout(Duration::from_secs(1)).is_ok());
        assert_eq!(count_hash_manifests(&app_context), 1);
    }
}
