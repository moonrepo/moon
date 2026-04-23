use moon_cas::{CasError, CasStore, ContentHash};
use moon_config::CacheCasConfig;
use starbase_sandbox::create_empty_sandbox;
use std::io::Cursor;

fn create_store(sandbox: &starbase_sandbox::Sandbox) -> CasStore {
    CasStore::new(sandbox.path().join("cas"), &CacheCasConfig::default()).unwrap()
}

fn create_verified_store(sandbox: &starbase_sandbox::Sandbox) -> CasStore {
    CasStore::new(
        sandbox.path().join("cas"),
        &CacheCasConfig {
            verify_integrity: true,
            ..Default::default()
        },
    )
    .unwrap()
}

mod cas {
    use super::*;

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
            assert!(store.contains_blob(&hash));
        }

        #[test]
        fn false_for_missing() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let hash = ContentHash::from_hex(&"0".repeat(64)).unwrap();
            assert!(!store.contains_blob(&hash));
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
            let path = store.object_path(&hash);

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
            let blob = store.object_path(&hash);

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
            assert!(store.contains_blob(&hash));

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

    mod concurrent_writes {
        use super::*;

        #[test]
        fn multiple_threads_same_content() {
            let sandbox = create_empty_sandbox();
            let store =
                CasStore::new(sandbox.path().join("cas"), &CacheCasConfig::default()).unwrap();

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
            assert!(store.contains_blob(&hash));
        }
    }
}
