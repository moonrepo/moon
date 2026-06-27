use moon_blob::{BlobContent, BlobInput, Bytes};
use moon_cache_local::LocalStorage;
use moon_cache_storage::{CacheContext, Manifest, ManifestFile, StorageBackend};
use moon_config::{CacheConfig, RemoteConfig};
use moon_hash::Digest;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::sync::Arc;

fn create_backend(sandbox: &Sandbox) -> Arc<LocalStorage> {
    let cache_dir = sandbox.path().join(".moon/cache");
    let context = CacheContext {
        cache_dir: cache_dir.clone(),
        cache_config: Arc::new(CacheConfig::default()),
        config_dir: sandbox.path().join(".moon"),
        remote_config: Arc::new(RemoteConfig::default()),
        remote_debug: false,
        workspace_root: sandbox.path().to_path_buf(),
    };

    Arc::new(LocalStorage::new(context, cache_dir, false).unwrap())
}

fn inline_source(content: &'static [u8]) -> BlobInput {
    BlobInput {
        content: BlobContent::Inline(Bytes::from_static(content)),
        digest: Digest::from_bytes(content).unwrap(),
    }
}

fn action_digest() -> Digest {
    Digest::from_bytes(b"action").unwrap()
}

mod local_storage {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_finds_and_retrieves_blobs() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);

        let sources = vec![
            inline_source(b"one"),
            inline_source(b"two"),
            inline_source(b"three"),
        ];
        let digests: Vec<Digest> = sources.iter().map(|source| source.digest.clone()).collect();

        let stored = Arc::clone(&backend)
            .store_blobs_batched(action_digest(), sources)
            .await
            .unwrap();
        assert_eq!(stored.len(), 3);

        // Everything is present now.
        let missing = Arc::clone(&backend)
            .find_missing_blobs_batched(action_digest(), digests.clone())
            .await
            .unwrap();
        assert!(missing.is_empty());

        let blobs = Arc::clone(&backend)
            .retrieve_blobs_batched(action_digest(), digests)
            .await
            .unwrap();
        assert_eq!(blobs.len(), 3);

        // Retrieval order across parallel chunks isn't guaranteed.
        let mut contents: Vec<Vec<u8>> = blobs
            .iter()
            .map(|blob| blob.content.get_bytes().unwrap().to_vec())
            .collect();
        contents.sort();
        assert_eq!(
            contents,
            vec![b"one".to_vec(), b"three".to_vec(), b"two".to_vec()]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn find_missing_returns_absent_digests_only() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);

        Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![inline_source(b"present")])
            .await
            .unwrap();

        let present = Digest::from_bytes(b"present").unwrap();
        let absent = Digest::from_bytes(b"absent").unwrap();

        let missing = Arc::clone(&backend)
            .find_missing_blobs_batched(action_digest(), vec![present, absent.clone()])
            .await
            .unwrap();

        assert_eq!(missing.len(), 1);
        assert!(missing.contains(&absent));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn storing_is_idempotent() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);

        Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![inline_source(b"dup")])
            .await
            .unwrap();

        // Re-storing short-circuits via the missing-blob check and reports the
        // full count as already present.
        let again = Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![inline_source(b"dup")])
            .await
            .unwrap();
        assert_eq!(again.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn chunks_and_round_trips_many_blobs() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);

        // Enough blobs to exercise chunk_into_batches spreading work across the
        // blocking pool rather than a single thread.
        let count: usize = 600;
        let sources: Vec<BlobInput> = (0..count)
            .map(|i| {
                let content = format!("blob-{i}").into_bytes();
                BlobInput {
                    digest: Digest::from_bytes(&content).unwrap(),
                    content: BlobContent::Inline(Bytes::from(content)),
                }
            })
            .collect();
        let digests: Vec<Digest> = sources.iter().map(|source| source.digest.clone()).collect();

        let stored = Arc::clone(&backend)
            .store_blobs_batched(action_digest(), sources)
            .await
            .unwrap();
        assert_eq!(stored.len(), count);

        let blobs = Arc::clone(&backend)
            .retrieve_blobs_batched(action_digest(), digests)
            .await
            .unwrap();
        assert_eq!(blobs.len(), count);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_file_backed_blob_sources() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("project/out.txt", "file blob content");
        let backend = create_backend(&sandbox);

        let content = b"file blob content";
        let digest = Digest::from_bytes(content).unwrap();
        let source = BlobInput {
            content: BlobContent::File("project/out.txt".into()),
            digest: digest.clone(),
        };

        let stored = Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![source])
            .await
            .unwrap();
        assert_eq!(stored.len(), 1);

        let blobs = Arc::clone(&backend)
            .retrieve_blobs_batched(action_digest(), vec![digest])
            .await
            .unwrap();
        assert_eq!(blobs[0].content.get_bytes().unwrap().to_vec(), content);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_and_retrieves_manifests() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);
        let key = Digest::from_bytes(b"manifest-key").unwrap();

        // Missing manifest reads back as None.
        assert!(
            backend
                .retrieve_manifest(key.clone())
                .await
                .unwrap()
                .is_none()
        );

        let manifest = Manifest {
            exit_code: 3,
            files: vec![ManifestFile {
                bytes: None,
                digest: Some(Digest::from_bytes(b"f").unwrap()),
                is_executable: true,
                path: "out/a.txt".into(),
                unix_mode: Some(0o755),
                ..Default::default()
            }],
            ..Default::default()
        };

        backend.store_manifest(key.clone(), manifest).await.unwrap();

        let loaded = backend
            .retrieve_manifest(key)
            .await
            .unwrap()
            .expect("manifest was stored");
        assert_eq!(loaded.exit_code, 3);
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].path.as_str(), "out/a.txt");
        assert!(loaded.files[0].is_executable);
        assert_eq!(loaded.files[0].unix_mode, Some(0o755));
    }
}
