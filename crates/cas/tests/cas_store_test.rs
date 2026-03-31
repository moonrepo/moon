use moon_cas::{CasError, CasStore, CasStoreConfig, ContentHash};
use starbase_sandbox::create_empty_sandbox;
use std::io::Cursor;
use std::time::Duration;

fn create_store(sandbox: &starbase_sandbox::Sandbox) -> CasStore {
    CasStore::new(sandbox.path().join("cas"), CasStoreConfig::default()).unwrap()
}

fn create_verified_store(sandbox: &starbase_sandbox::Sandbox) -> CasStore {
    CasStore::new(
        sandbox.path().join("cas"),
        CasStoreConfig {
            verify_on_read: true,
            ..Default::default()
        },
    )
    .unwrap()
}

mod content_hash {
    use super::*;

    #[test]
    fn valid_hex() {
        let hex = "a".repeat(64);
        let hash = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(hash.as_hex(), hex);
        assert_eq!(hash.prefix(), "aa");
        assert_eq!(hash.suffix().len(), 62);
    }

    #[test]
    fn rejects_short_hex() {
        let result = ContentHash::from_hex("abcd");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_hex() {
        let hex = "g".repeat(64);
        let result = ContentHash::from_hex(&hex);
        assert!(result.is_err());
    }

    #[test]
    fn normalizes_to_lowercase() {
        let hex = "A".repeat(64);
        let hash = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(hash.as_hex(), "a".repeat(64));
    }

    #[test]
    fn display_shows_hex() {
        let hex = "b".repeat(64);
        let hash = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(format!("{hash}"), hex);
    }
}

mod write_bytes {
    use super::*;

    #[test]
    fn round_trip() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let data = b"hello world";
        let hash = store.write_bytes(data).unwrap();
        let read_back = store.read_bytes(&hash).unwrap();

        assert_eq!(read_back, data);
    }

    #[test]
    fn empty_content() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"").unwrap();
        let read_back = store.read_bytes(&hash).unwrap();

        assert!(read_back.is_empty());
    }

    #[test]
    fn idempotent() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let data = b"duplicate content";
        let hash1 = store.write_bytes(data).unwrap();
        let hash2 = store.write_bytes(data).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn different_content_different_hash() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash1 = store.write_bytes(b"aaa").unwrap();
        let hash2 = store.write_bytes(b"bbb").unwrap();

        assert_ne!(hash1, hash2);
    }
}

mod write_file {
    use super::*;

    #[test]
    fn round_trip() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let source = sandbox.path().join("input.txt");
        std::fs::write(&source, b"file content").unwrap();

        let hash = store.write_file(&source).unwrap();
        let read_back = store.read_bytes(&hash).unwrap();

        assert_eq!(read_back, b"file content");
    }

    #[test]
    fn matches_write_bytes() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let data = b"same content";
        let source = sandbox.path().join("input.txt");
        std::fs::write(&source, data).unwrap();

        let hash_bytes = store.write_bytes(data).unwrap();
        let hash_file = store.write_file(&source).unwrap();

        assert_eq!(hash_bytes, hash_file);
    }
}

mod write_stream {
    use super::*;

    #[test]
    fn round_trip() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let data = b"stream content";
        let cursor = Cursor::new(data);

        let hash = store.write_stream(cursor).unwrap();
        let read_back = store.read_bytes(&hash).unwrap();

        assert_eq!(read_back, data.as_slice());
    }

    #[test]
    fn matches_write_bytes() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let data = b"consistent hashing";
        let hash_bytes = store.write_bytes(data).unwrap();
        let hash_stream = store.write_stream(Cursor::new(data)).unwrap();

        assert_eq!(hash_bytes, hash_stream);
    }
}

mod contains {
    use super::*;

    #[test]
    fn true_after_write() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"exists").unwrap();
        assert!(store.contains(&hash));
    }

    #[test]
    fn false_for_missing() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = ContentHash::from_hex(&"0".repeat(64)).unwrap();
        assert!(!store.contains(&hash));
    }
}

mod blob_path {
    use super::*;

    #[test]
    fn returns_some_after_write() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"blob").unwrap();
        let path = store.blob_path(&hash);

        assert!(path.is_some());
        assert!(path.unwrap().exists());
    }

    #[test]
    fn returns_none_for_missing() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = ContentHash::from_hex(&"f".repeat(64)).unwrap();
        assert!(store.blob_path(&hash).is_none());
    }

    #[test]
    fn shard_layout() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"shard test").unwrap();
        let path = store.blob_path(&hash).unwrap();

        // Path should be: <root>/objects/<2-char prefix>/<62-char suffix>
        let parent = path.parent().unwrap();
        let shard_name = parent.file_name().unwrap().to_str().unwrap();
        assert_eq!(shard_name.len(), 2);
        assert_eq!(shard_name, hash.prefix());

        let file_name = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(file_name, hash.suffix());
    }
}

mod read_bytes {
    use super::*;

    #[test]
    fn not_found() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = ContentHash::from_hex(&"1".repeat(64)).unwrap();
        let result = store.read_bytes(&hash);

        assert!(result.is_err());
        let err = result.unwrap_err();
        let cas_err = err.downcast_ref::<CasError>().unwrap();
        assert!(matches!(cas_err, CasError::NotFound { .. }));
    }
}

mod integrity {
    use super::*;

    #[test]
    fn passes_for_valid_blob() {
        let sandbox = create_empty_sandbox();
        let store = create_verified_store(&sandbox);

        let hash = store.write_bytes(b"valid content").unwrap();
        let result = store.read_bytes(&hash);

        assert!(result.is_ok());
    }

    #[test]
    fn detects_corruption() {
        let sandbox = create_empty_sandbox();
        let store = create_verified_store(&sandbox);

        let hash = store.write_bytes(b"original content").unwrap();
        let path = store.blob_path(&hash).unwrap();

        // Corrupt the blob on disk (need to make writable first).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut perms = std::fs::metadata(&path).unwrap().permissions();
            perms.set_readonly(false);
            std::fs::set_permissions(&path, perms).unwrap();
        }
        std::fs::write(&path, b"corrupted!").unwrap();

        let result = store.read_bytes(&hash);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let cas_err = err.downcast_ref::<CasError>().unwrap();
        assert!(matches!(cas_err, CasError::IntegrityMismatch { .. }));
    }
}

mod link_to {
    use super::*;

    #[test]
    fn creates_file_at_dest() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let data = b"link target content";
        let hash = store.write_bytes(data).unwrap();

        let dest = sandbox.path().join("output.txt");
        store.link_to(&hash, &dest).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), data);
    }

    #[test]
    fn creates_parent_dirs() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"nested").unwrap();
        let dest = sandbox.path().join("deep/nested/dir/output.txt");
        store.link_to(&hash, &dest).unwrap();

        assert!(dest.exists());
    }

    #[cfg(unix)]
    #[test]
    fn shares_inode_on_same_device() {
        use std::os::unix::fs::MetadataExt;

        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"inode test").unwrap();
        let blob = store.blob_path(&hash).unwrap();

        let dest = sandbox.path().join("linked.txt");
        store.link_to(&hash, &dest).unwrap();

        let blob_ino = std::fs::metadata(&blob).unwrap().ino();
        let dest_ino = std::fs::metadata(&dest).unwrap().ino();
        assert_eq!(blob_ino, dest_ino);
    }

    #[test]
    fn not_found() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = ContentHash::from_hex(&"2".repeat(64)).unwrap();
        let dest = sandbox.path().join("nope.txt");
        let result = store.link_to(&hash, &dest);

        assert!(result.is_err());
    }
}

mod link_from {
    use super::*;

    #[test]
    fn ingests_file() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let source = sandbox.path().join("source.txt");
        std::fs::write(&source, b"ingest me").unwrap();

        let hash = store.link_from(&source).unwrap();
        assert!(store.contains(&hash));

        let read_back = store.read_bytes(&hash).unwrap();
        assert_eq!(read_back, b"ingest me");
    }

    #[test]
    fn hash_matches_write_bytes() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let data = b"consistent hash";
        let source = sandbox.path().join("source.txt");
        std::fs::write(&source, data).unwrap();

        let hash_link = store.link_from(&source).unwrap();
        let hash_bytes = store.write_bytes(data).unwrap();

        assert_eq!(hash_link, hash_bytes);
    }
}

mod gc {
    use super::*;

    #[test]
    fn removes_stale_blobs() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"stale").unwrap();
        let path = store.blob_path(&hash).unwrap();

        // Backdate the mtime to 2 hours ago.
        backdate_mtime(&path, Duration::from_secs(7200));

        let result = store.gc(Duration::from_secs(3600)).unwrap();
        assert_eq!(result.blobs_removed, 1);
        assert!(!store.contains(&hash));
    }

    #[test]
    fn preserves_fresh_blobs() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"fresh").unwrap();

        let result = store.gc(Duration::from_secs(3600)).unwrap();
        assert_eq!(result.blobs_removed, 0);
        assert!(store.contains(&hash));
    }

    #[test]
    fn touch_prevents_gc() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        let hash = store.write_bytes(b"touched").unwrap();
        let path = store.blob_path(&hash).unwrap();

        // Backdate, then touch.
        backdate_mtime(&path, Duration::from_secs(7200));
        store.touch(&hash).unwrap();

        let result = store.gc(Duration::from_secs(3600)).unwrap();
        assert_eq!(result.blobs_removed, 0);
        assert!(store.contains(&hash));
    }

    #[test]
    fn purge_removes_all() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        store.write_bytes(b"one").unwrap();
        store.write_bytes(b"two").unwrap();
        store.write_bytes(b"three").unwrap();

        let result = store.purge().unwrap();
        assert_eq!(result.blobs_removed, 3);
    }

    #[test]
    fn cleans_orphaned_temp_files() {
        let sandbox = create_empty_sandbox();
        let store = create_store(&sandbox);

        // Simulate an orphaned temp file.
        let orphan = sandbox.path().join("cas/tmp/orphan");
        std::fs::write(&orphan, b"orphan").unwrap();
        backdate_mtime(&orphan, Duration::from_secs(7200));

        store.gc(Duration::from_secs(86400)).unwrap();
        assert!(!orphan.exists());
    }

    fn backdate_mtime(path: &std::path::Path, age: Duration) {
        // Make writable first (blobs are read-only).
        let metadata = std::fs::metadata(path).unwrap();
        let mut perms = metadata.permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(path, perms).unwrap();

        let past = std::time::SystemTime::now() - age;
        let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(past).unwrap();

        // Restore read-only.
        let mut perms = file.metadata().unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(path, perms).unwrap();
    }
}

mod concurrent_writes {
    use super::*;

    #[test]
    fn multiple_threads_same_content() {
        let sandbox = create_empty_sandbox();
        let store = CasStore::new(sandbox.path().join("cas"), CasStoreConfig::default()).unwrap();

        let data = b"concurrent content";

        std::thread::scope(|s| {
            let handles: Vec<_> = (0..8)
                .map(|_| s.spawn(|| store.write_bytes(data).unwrap()))
                .collect();

            let hashes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

            // All threads should produce the same hash.
            let first = &hashes[0];
            for hash in &hashes[1..] {
                assert_eq!(hash, first);
            }
        });

        // Only one blob on disk.
        let hash = store.write_bytes(data).unwrap();
        assert!(store.contains(&hash));
    }
}
