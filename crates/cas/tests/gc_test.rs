use moon_cas::CasStore;
use moon_config::CacheCasConfig;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::time::Duration;

fn create_store(sandbox: &Sandbox) -> CasStore {
    CasStore::new(sandbox.path().join("cas"), &CacheCasConfig::default()).unwrap()
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

        let hash = store.write_bytes(b"stale").unwrap();
        let path = store.object_path(&hash);

        // Backdate the mtime to 2 hours ago.
        backdate_mtime(&path, Duration::from_secs(7200));

        let result = store.gc(Duration::from_secs(3600)).await.unwrap();

        assert_eq!(result.blobs_removed, 1);
        assert!(!store.contains_object(&hash).unwrap());
    }

    #[tokio::test]
    async fn preserves_fresh_blobs() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"fresh").unwrap();
        let result = store.gc(Duration::from_secs(3600)).await.unwrap();

        assert_eq!(result.blobs_removed, 0);
        assert!(store.contains_object(&hash).unwrap());
    }

    #[tokio::test]
    async fn touch_prevents_gc() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"touched").unwrap();
        let path = store.object_path(&hash);

        // Backdate, then touch.
        backdate_mtime(&path, Duration::from_secs(7200));
        store.touch(&hash).unwrap();

        let result = store.gc(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(result.blobs_removed, 0);
        assert!(store.contains_object(&hash).unwrap());
    }

    #[tokio::test]
    async fn purge_removes_all() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        store.write_bytes(b"one").unwrap();
        store.write_bytes(b"two").unwrap();
        store.write_bytes(b"three").unwrap();

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
