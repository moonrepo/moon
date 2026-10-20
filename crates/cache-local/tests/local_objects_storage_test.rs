use moon_blob::{BlobContent, BlobInput, Bytes};
use moon_cache_local::{LocalObjectsStorage, LocalStorage};
use moon_cache_storage::{
    CacheContext, ObjectsStorage, StorageBackend, TaskManifest, TaskManifestFile,
};
use moon_config::CacheConfig;
use moon_hash::Digest;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const WEEK: Duration = Duration::from_secs(86400 * 7);

fn create_backend(sandbox: &Sandbox) -> Arc<LocalObjectsStorage> {
    Arc::new(LocalObjectsStorage::new(CacheContext::new(sandbox.path())).unwrap())
}

fn create_storage(sandbox: &Sandbox) -> ObjectsStorage {
    let context = CacheContext::new(sandbox.path());

    let mut storage = ObjectsStorage::new(context.clone());
    storage.add_local_backend(LocalObjectsStorage::new(context).unwrap());
    storage
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

fn backdate(path: &Path, age: Duration) {
    let past = SystemTime::now() - age;
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(past).unwrap();
}

fn object_path(sandbox: &Sandbox, digest: &Digest) -> PathBuf {
    sandbox
        .path()
        .join(".moon/cache/objects")
        .join(digest.hash.prefix())
        .join(digest.hash.suffix())
}

fn blob_path(sandbox: &Sandbox, digest: &Digest) -> PathBuf {
    sandbox
        .path()
        .join(".moon/cache/blobs")
        .join(digest.hash.prefix())
        .join(digest.hash.suffix())
}

fn manifest_referencing(digest: &Digest) -> TaskManifest {
    TaskManifest {
        files: vec![TaskManifestFile {
            digest: Some(digest.clone()),
            path: "out.txt".into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

mod local_objects_storage {
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
        assert_eq!(stored.digests.len(), 3);

        let missing = Arc::clone(&backend)
            .find_missing_blobs_batched(action_digest(), digests.clone())
            .await
            .unwrap();
        assert!(missing.is_empty());

        let blobs = Arc::clone(&backend)
            .retrieve_blobs_batched(action_digest(), digests)
            .await
            .unwrap();
        assert_eq!(blobs.blobs.len(), 3);

        // Objects are handed back as paths into the objects store, not read
        // into memory.
        for blob in &blobs.blobs {
            let BlobContent::File(path) = &blob.content else {
                panic!("expected a file-backed object");
            };

            assert_eq!(path, &object_path(&sandbox, &blob.digest));
        }

        // Retrieval order across parallel batches isn't guaranteed.
        let mut contents: Vec<Vec<u8>> = blobs
            .blobs
            .iter()
            .map(|blob| blob.content.read_bytes().unwrap())
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

        assert_eq!(missing, vec![absent]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn retrieves_only_present_blobs() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);

        Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![inline_source(b"present")])
            .await
            .unwrap();

        let present = Digest::from_bytes(b"present").unwrap();
        let absent = Digest::from_bytes(b"absent").unwrap();

        let blobs = Arc::clone(&backend)
            .retrieve_blobs_batched(action_digest(), vec![present.clone(), absent])
            .await
            .unwrap();

        assert_eq!(blobs.blobs.len(), 1);
        assert_eq!(blobs.blobs[0].digest, present);
        assert!(!blobs.success, "a partial retrieval is not a success");
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

        assert!(again.success);
        assert_eq!(again.digests.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_file_backed_blob_sources() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("project/out.txt", "file blob content");
        let backend = create_backend(&sandbox);

        let digest = Digest::from_bytes(b"file blob content").unwrap();
        let source = BlobInput {
            content: BlobContent::File(sandbox.path().join("project/out.txt")),
            digest: digest.clone(),
        };

        let stored = Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![source])
            .await
            .unwrap();
        assert_eq!(stored.digests.len(), 1);

        let blobs = Arc::clone(&backend)
            .retrieve_blobs_batched(action_digest(), vec![digest])
            .await
            .unwrap();

        assert_eq!(blobs.blobs.len(), 1);
        assert_eq!(
            blobs.blobs[0].content.read_bytes().unwrap(),
            b"file blob content"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finds_blobs_by_prefix() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);

        let source = inline_source(b"prefixed");
        let digest = source.digest.clone();

        Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![source])
            .await
            .unwrap();

        assert_eq!(
            backend
                .find_blobs_by_prefix(&digest.hash.as_str()[0..8])
                .await
                .unwrap(),
            vec![digest]
        );
        assert!(
            backend
                .find_blobs_by_prefix("ffffffffff")
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn gc_keeps_objects_within_lifetime() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);

        let source = inline_source(b"recent");
        let digest = source.digest.clone();

        Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![source])
            .await
            .unwrap();

        backdate(&object_path(&sandbox, &digest), Duration::from_secs(7200));

        let stats = backend.gc(WEEK).await.unwrap();

        assert_eq!(stats.blobs_removed, 0);
        assert!(
            backend
                .find_missing_blobs(vec![digest])
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn gc_removes_objects_past_lifetime() {
        let sandbox = create_empty_sandbox();
        let backend = create_backend(&sandbox);

        let source = inline_source(b"stale");
        let digest = source.digest.clone();

        Arc::clone(&backend)
            .store_blobs_batched(action_digest(), vec![source])
            .await
            .unwrap();

        backdate(&object_path(&sandbox, &digest), WEEK * 2);

        let stats = backend.gc(WEEK).await.unwrap();

        assert_eq!(stats.blobs_removed, 1);
        assert_eq!(stats.bytes_saved, 5);
        assert_eq!(
            backend
                .find_missing_blobs(vec![digest.clone()])
                .await
                .unwrap(),
            vec![digest]
        );
    }
}

mod hash_manifests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_in_the_objects_dir() {
        // Hash manifests live in their own store, apart from the task output
        // blobs that action cache GC sweeps.
        let sandbox = create_empty_sandbox();
        let storage = create_storage(&sandbox);

        let digest = storage
            .store_hash_manifest("task", vec!["command", "args"])
            .await
            .unwrap();

        assert!(object_path(&sandbox, &digest).exists());
        assert!(!blob_path(&sandbox, &digest).exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_a_stored_manifest_from_disk() {
        let sandbox = create_empty_sandbox();
        let storage = create_storage(&sandbox);

        let digest = storage
            .store_hash_manifest("task", vec!["command", "args"])
            .await
            .unwrap();

        let content = storage
            .load_hash_manifest(&digest)
            .await
            .unwrap()
            .expect("the manifest reads back");

        assert_eq!(content.read_bytes().unwrap(), br#"[["command","args"]]"#);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn persists_across_storage_instances() {
        // A hash manifest marks work as done for later runs, each of which
        // opens its own storage over the same cache directory.
        let sandbox = create_empty_sandbox();

        let digest = create_storage(&sandbox)
            .store_hash_manifest("task", vec!["command", "args"])
            .await
            .unwrap();

        assert!(create_storage(&sandbox).has_hash_manifest(&digest).await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_a_digest_from_a_partial_hash() {
        let sandbox = create_empty_sandbox();
        let storage = create_storage(&sandbox);

        let digest = storage
            .store_hash_manifest("task", vec!["command", "args"])
            .await
            .unwrap();

        let found = storage.get_backends()[0]
            .find_blobs_by_prefix(&digest.hash.as_str()[0..8])
            .await
            .unwrap();

        assert_eq!(found, vec![digest]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn clean_removes_stale_manifests_only() {
        // A manifest past the lifetime is swept, so the work it marked as done
        // runs again next time, while a recent one keeps short-circuiting.
        let sandbox = create_empty_sandbox();
        let storage = create_storage(&sandbox);

        let stale = storage
            .store_hash_manifest("task", vec!["stale"])
            .await
            .unwrap();
        let fresh = storage
            .store_hash_manifest("task", vec!["fresh"])
            .await
            .unwrap();

        backdate(&object_path(&sandbox, &stale), WEEK * 2);

        let stats = storage.clean(WEEK).await.unwrap();

        assert_eq!(stats.blobs_removed, 1);
        assert!(!storage.has_hash_manifest(&stale).await);
        assert!(storage.has_hash_manifest(&fresh).await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn is_not_swept_by_action_cache_gc() {
        let sandbox = create_empty_sandbox();
        let storage = create_storage(&sandbox);

        let mut config = CacheConfig::default();
        config.cas.max_size = Some("1b".into());

        let mut context = CacheContext::new(sandbox.path());
        context.cache_config = Arc::new(config);

        let cache_dir = context.cache_dir.clone();
        let actions = Arc::new(LocalStorage::new(context, cache_dir).unwrap());

        // The task manifest is keyed by the same digest as the hash manifest
        let digest = storage
            .store_hash_manifest("task", vec!["command", "args"])
            .await
            .unwrap();
        let output = inline_source(b"task output");
        let output_digest = output.digest.clone();

        Arc::clone(&actions)
            .store_blobs_batched(digest.clone(), vec![output])
            .await
            .unwrap();
        actions
            .store_task_manifest(digest.clone(), manifest_referencing(&output_digest))
            .await
            .unwrap();

        backdate(&object_path(&sandbox, &digest), WEEK * 2);
        backdate(
            &blob_path(&sandbox, &output_digest),
            Duration::from_secs(7200),
        );

        actions.gc(WEEK).await.unwrap();

        // The task manifest is evicted over the size budget and its output
        // swept, while the hash manifest is untouched, even past the lifetime
        assert!(
            actions
                .retrieve_task_manifest(digest.clone())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            actions
                .find_missing_blobs(vec![output_digest.clone()])
                .await
                .unwrap(),
            vec![output_digest],
        );
        assert!(storage.has_hash_manifest(&digest).await);
    }
}
