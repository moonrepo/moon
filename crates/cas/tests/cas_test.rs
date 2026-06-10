use moon_blob::Blob;
use moon_cas::{CasError, CasStore};
use moon_config::CacheCasConfig;
use moon_hash::ContentHash;
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
            let digest = store.write_bytes(data).unwrap();
            let read_back = store.read_bytes(&digest.hash).unwrap();

            assert_eq!(read_back, data);
        }

        #[test]
        fn populates_digest_size() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let data = b"sized payload";
            let digest = store.write_bytes(data).unwrap();

            assert_eq!(digest.size, data.len() as i64);
        }

        #[test]
        fn empty_content() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let digest = store.write_bytes(b"").unwrap();
            let read_back = store.read_bytes(&digest.hash).unwrap();

            assert!(read_back.is_empty());
            assert_eq!(digest.size, 0);
        }

        #[test]
        fn idempotent() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let data = b"duplicate content";
            let digest1 = store.write_bytes(data).unwrap();
            let digest2 = store.write_bytes(data).unwrap();

            assert_eq!(digest1, digest2);
        }

        #[test]
        fn different_content_different_hash() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let digest1 = store.write_bytes(b"aaa").unwrap();
            let digest2 = store.write_bytes(b"bbb").unwrap();

            assert_ne!(digest1, digest2);
        }
    }

    mod write_blob {
        use super::*;

        #[test]
        fn round_trip() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let blob = Blob::from_bytes(b"blob content".to_vec()).unwrap();

            store.write_blob(&blob).unwrap();

            let read_back = store.read_bytes(&blob.digest.hash).unwrap();

            assert_eq!(read_back, b"blob content");
        }

        #[test]
        fn idempotent() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let blob = Blob::from_bytes(b"twice".to_vec()).unwrap();

            store.write_blob(&blob).unwrap();
            store.write_blob(&blob).unwrap();

            assert!(store.contains_object(&blob.digest.hash));
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

            let blob = store.write_file(&source).unwrap();
            let read_back = store.read_bytes(&blob.digest.hash).unwrap();

            assert_eq!(read_back, b"file content");
        }

        #[test]
        fn matches_write_bytes() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let data = b"same content";
            let source = sandbox.path().join("input.txt");
            std::fs::write(&source, data).unwrap();

            let digest_bytes = store.write_bytes(data).unwrap();
            let blob = store.write_file(&source).unwrap();

            assert_eq!(digest_bytes.hash, blob.digest.hash);
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

            let digest = store.write_stream(cursor).unwrap();
            let read_back = store.read_bytes(&digest.hash).unwrap();

            assert_eq!(read_back, data.as_slice());
        }

        #[test]
        fn populates_digest_size() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let data = b"sized stream";
            let digest = store.write_stream(Cursor::new(data)).unwrap();

            assert_eq!(digest.size, data.len() as i64);
        }

        #[test]
        fn matches_write_bytes() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let data = b"consistent hashing";
            let digest_bytes = store.write_bytes(data).unwrap();
            let digest_stream = store.write_stream(Cursor::new(data)).unwrap();

            assert_eq!(digest_bytes, digest_stream);
        }

        #[test]
        fn handles_payload_larger_than_buffer() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            // The streaming hasher reads in 64 KiB chunks. Use a payload that
            // spans several chunks (and isn't an exact multiple) to exercise
            // the loop's partial-final-read branch.
            let data: Vec<u8> = (0..(64 * 1024 * 3 + 137))
                .map(|i| (i % 251) as u8)
                .collect();

            let digest_stream = store.write_stream(Cursor::new(&data)).unwrap();
            let digest_bytes = store.write_bytes(&data).unwrap();

            assert_eq!(digest_stream, digest_bytes);
            assert_eq!(digest_stream.size, data.len() as i64);

            let read_back = store.read_bytes(&digest_stream.hash).unwrap();
            assert_eq!(read_back, data);
        }

        #[test]
        fn empty_stream() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let digest = store.write_stream(Cursor::new(b"")).unwrap();

            assert_eq!(digest.size, 0);
            assert!(store.contains_object(&digest.hash));
        }
    }

    mod contains {
        use super::*;

        #[test]
        fn true_after_write() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let digest = store.write_bytes(b"exists").unwrap();
            assert!(store.contains_object(&digest.hash));
        }

        #[test]
        fn false_for_missing() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let hash = ContentHash::from_hex("0".repeat(64)).unwrap();
            assert!(!store.contains_object(&hash));
        }

        #[test]
        fn pure_existence_check_does_not_verify_content() {
            // contains_object is documented as a pure existence check —
            // even with verify_integrity enabled, it must NOT rehash the
            // existing file. Verification happens lazily on read.
            let sandbox = create_empty_sandbox();
            let store = create_verified_store(&sandbox);

            let digest = store.write_bytes(b"original").unwrap();

            // Corrupt the on-disk blob.
            std::fs::write(store.object_path(&digest.hash), b"tampered").unwrap();

            // contains_object should still return true; it doesn't read content.
            assert!(store.contains_object(&digest.hash));
        }

        #[test]
        fn does_not_auto_delete_corrupt_blob() {
            // The previous implementation removed corrupt blobs from inside
            // contains_object. The optimized version leaves them alone — the
            // file still exists after the check, and the next read_bytes is
            // what surfaces the integrity error.
            let sandbox = create_empty_sandbox();
            let store = create_verified_store(&sandbox);

            let digest = store.write_bytes(b"original").unwrap();
            let path = store.object_path(&digest.hash);

            std::fs::write(&path, b"tampered").unwrap();

            // Touch the existence check; it must be a no-op on disk.
            let _ = store.contains_object(&digest.hash);

            assert!(path.exists());
        }
    }

    mod write {
        use super::*;

        #[test]
        fn returns_true_for_new_blob() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let data = b"fresh";
            let hash = ContentHash::hash_bytes(data).unwrap();

            assert!(store.write(&hash, data).unwrap());
        }

        #[test]
        fn returns_false_when_already_present() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let data = b"already there";
            let hash = ContentHash::hash_bytes(data).unwrap();

            // First write commits to disk.
            assert!(store.write(&hash, data).unwrap());

            // Second write short-circuits — the existence check makes it a no-op.
            assert!(!store.write(&hash, data).unwrap());
        }
    }

    mod read_bytes {
        use super::*;

        #[test]
        fn not_found() {
            let sandbox = create_empty_sandbox();
            let store = create_store(&sandbox);

            let hash = ContentHash::from_hex("1".repeat(64)).unwrap();
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

            let digest = store.write_bytes(b"valid content").unwrap();
            let result = store.read_bytes(&digest.hash);

            assert!(result.is_ok());
        }

        #[test]
        fn detects_corruption() {
            let sandbox = create_empty_sandbox();
            let store = create_verified_store(&sandbox);

            let digest = store.write_bytes(b"original content").unwrap();
            let path = store.object_path(&digest.hash);

            std::fs::write(&path, b"corrupted!").unwrap();

            let result = store.read_bytes(&digest.hash);
            assert!(result.is_err());
            let err = result.unwrap_err();
            let cas_err = err.downcast_ref::<CasError>().unwrap();
            assert!(matches!(cas_err, CasError::IntegrityMismatch { .. }));
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

                let digests: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

                // All threads should produce the same digest.
                let first = &digests[0];
                for digest in &digests[1..] {
                    assert_eq!(digest, first);
                }
            });

            // Only one blob on disk.
            let digest = store.write_bytes(data).unwrap();
            assert!(store.contains_object(&digest.hash));
        }
    }
}
