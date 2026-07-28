use moon_cas::CasStore;
use moon_config::CacheCasConfig;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::time::Duration;

fn create_store(sandbox: &Sandbox) -> CasStore {
    CasStore::new(sandbox.path().join("cas"), CacheCasConfig::default()).unwrap()
}

fn backdate_mtime(path: &std::path::Path, age: Duration) {
    let past = std::time::SystemTime::now() - age;
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(past).unwrap();
}

mod gc {
    use super::*;

    #[tokio::test]
    async fn removes_stale_blobs() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let digest = store.store_bytes(b"stale").unwrap();
        let path = store.object_path(&digest.hash);

        // Backdate the mtime to 2 hours ago.
        backdate_mtime(&path, Duration::from_secs(7200));

        let result = store.gc(Duration::from_secs(3600)).await.unwrap();

        assert_eq!(result.blobs_removed, 1);
        assert!(!store.contains_object(&digest.hash));
    }

    #[tokio::test]
    async fn preserves_fresh_blobs() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let digest = store.store_bytes(b"fresh").unwrap();
        let result = store.gc(Duration::from_secs(3600)).await.unwrap();

        assert_eq!(result.blobs_removed, 0);
        assert!(store.contains_object(&digest.hash));
    }

    #[tokio::test]
    async fn touch_prevents_gc() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let digest = store.store_bytes(b"touched").unwrap();
        let path = store.object_path(&digest.hash);

        // Backdate, then touch.
        backdate_mtime(&path, Duration::from_secs(7200));
        store.touch(&digest.hash).unwrap();

        let result = store.gc(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(result.blobs_removed, 0);
        assert!(store.contains_object(&digest.hash));
    }

    #[tokio::test]
    async fn purge_removes_all() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        store.store_bytes(b"one").unwrap();
        store.store_bytes(b"two").unwrap();
        store.store_bytes(b"three").unwrap();

        let result = store.purge().await.unwrap();

        assert_eq!(result.blobs_removed, 3);
    }

    #[tokio::test]
    async fn cleans_orphaned_temp_files() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        // Simulate an orphaned temp file.
        let orphan = sandbox.path().join("cas/temp/orphan");

        std::fs::write(&orphan, b"orphan").unwrap();
        backdate_mtime(&orphan, Duration::from_secs(7200));

        store.gc(Duration::from_secs(86400)).await.unwrap();

        assert!(!orphan.exists());
    }
}

mod retain {
    use super::*;
    use moon_hash::ContentHash;
    use rustc_hash::FxHashSet;
    use std::sync::Arc;

    fn keep_set(hashes: &[&ContentHash]) -> Arc<FxHashSet<ContentHash>> {
        Arc::new(hashes.iter().map(|hash| (*hash).clone()).collect())
    }

    #[tokio::test]
    async fn keeps_referenced_and_sweeps_unreferenced() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let referenced = store.store_bytes(b"referenced").unwrap();
        let orphan = store.store_bytes(b"orphan").unwrap();

        // Age both past the grace window so the sweep is driven by reachability.
        backdate_mtime(
            &store.object_path(&referenced.hash),
            Duration::from_secs(7200),
        );
        backdate_mtime(&store.object_path(&orphan.hash), Duration::from_secs(7200));

        let result = store
            .retain(keep_set(&[&referenced.hash]), Duration::from_secs(3600))
            .await
            .unwrap();

        assert_eq!(result.blobs_removed, 1);
        assert!(store.contains_object(&referenced.hash));
        assert!(!store.contains_object(&orphan.hash));
    }

    #[tokio::test]
    async fn grace_spares_recently_written_unreferenced_blobs() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        // Freshly written and unreferenced — a blob mid-ingest whose manifest
        // hasn't landed yet must survive the grace window.
        let pending = store.store_bytes(b"pending").unwrap();

        let result = store
            .retain(keep_set(&[]), Duration::from_secs(3600))
            .await
            .unwrap();

        assert_eq!(result.blobs_removed, 0);
        assert!(store.contains_object(&pending.hash));
    }

    #[tokio::test]
    async fn sweeps_unreferenced_blobs_past_grace() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let orphan = store.store_bytes(b"orphan").unwrap();
        backdate_mtime(&store.object_path(&orphan.hash), Duration::from_secs(7200));

        let result = store
            .retain(keep_set(&[]), Duration::from_secs(3600))
            .await
            .unwrap();

        assert_eq!(result.blobs_removed, 1);
        assert!(!store.contains_object(&orphan.hash));
    }
}
