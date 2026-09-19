use async_trait::async_trait;
use moon_blob::{BlobContent, BlobInput, BlobOutput, Bytes};
use moon_cache_storage::{
    CacheCapabilities, CacheContext, Storage, StorageBackend, StorageOptions,
};
use moon_common::Id;
use moon_hash::{ContentHash, ContentHasher, Digest};
use moon_manifest::{TaskManifest, TaskManifestFile};
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_utils::json::serde_json;
use std::path::Path;
use std::sync::{Arc, Mutex};

fn create_storage() -> Storage {
    Storage::new(CacheContext::new(Path::new("/moon-test")))
}

/// In-memory backend with externally inspectable maps, so tests can both seed
/// state and assert on what was written back.
#[derive(Debug)]
struct MemoryBackend {
    id: Id,
    capabilities: CacheCapabilities,
    blobs: Arc<Mutex<FxHashMap<Digest, Bytes>>>,
    manifests: Arc<Mutex<FxHashMap<Digest, TaskManifest>>>,

    // Failure injection for the abort/degrade paths.
    fail_find_missing: bool,
    fail_store_blobs: bool,
    fail_retrieve_blobs: bool,

    // A read-only tier (e.g. a shared cache the user can't write) must never
    // be a warm target.
    read_only: bool,
}

impl MemoryBackend {
    fn new(id: &str) -> Self {
        Self {
            id: Id::raw(id),
            capabilities: CacheCapabilities::default(),
            blobs: Arc::new(Mutex::new(FxHashMap::default())),
            manifests: Arc::new(Mutex::new(FxHashMap::default())),
            fail_find_missing: false,
            fail_store_blobs: false,
            fail_retrieve_blobs: false,
            read_only: false,
        }
    }

    fn failing_find_missing(mut self) -> Self {
        self.fail_find_missing = true;
        self
    }

    fn failing_store_blobs(mut self) -> Self {
        self.fail_store_blobs = true;
        self
    }

    fn failing_retrieve_blobs(mut self) -> Self {
        self.fail_retrieve_blobs = true;
        self
    }

    fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

#[async_trait]
impl StorageBackend for MemoryBackend {
    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_capabilities(&self) -> &CacheCapabilities {
        &self.capabilities
    }

    fn is_readable(&self) -> bool {
        true
    }

    fn is_writable(&self) -> bool {
        !self.read_only
    }

    async fn retrieve_task_manifest(&self, digest: Digest) -> miette::Result<Option<TaskManifest>> {
        Ok(self.manifests.lock().unwrap().get(&digest).cloned())
    }

    async fn store_task_manifest(
        &self,
        digest: Digest,
        mut manifest: TaskManifest,
    ) -> miette::Result<()> {
        // Mimic a serializing backend: byte fields are #[serde(skip)], so a
        // persisted manifest returns without inline bytes and must be
        // re-hydrated from the stored blobs.
        manifest.stderr_bytes = None;
        manifest.stdout_bytes = None;
        for file in &mut manifest.files {
            file.bytes = None;
        }

        self.manifests.lock().unwrap().insert(digest, manifest);

        Ok(())
    }

    async fn find_blobs_by_prefix(&self, prefix: &str) -> miette::Result<Vec<Digest>> {
        Ok(self
            .blobs
            .lock()
            .unwrap()
            .keys()
            .filter(|digest| digest.hash.as_str().starts_with(prefix))
            .cloned()
            .collect())
    }

    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>> {
        if self.fail_find_missing {
            return Err(miette::miette!("simulated find_missing failure"));
        }

        let blobs = self.blobs.lock().unwrap();

        Ok(blob_digests
            .into_iter()
            .filter(|digest| !blobs.contains_key(digest))
            .collect())
    }

    async fn store_blobs(
        &self,
        blob_sources: Vec<BlobInput>,
        _stream: bool,
    ) -> miette::Result<Vec<Digest>> {
        if self.fail_store_blobs {
            return Err(miette::miette!("simulated store_blobs failure"));
        }

        let mut blobs = self.blobs.lock().unwrap();
        let mut stored = vec![];

        for source in blob_sources {
            if let BlobContent::Inline(bytes) = source.content {
                blobs.insert(source.digest.clone(), bytes);
                stored.push(source.digest);
            }
        }

        Ok(stored)
    }

    async fn retrieve_blobs(
        &self,
        blob_digests: Vec<Digest>,
        _stream: bool,
    ) -> miette::Result<Vec<BlobOutput>> {
        if self.fail_retrieve_blobs {
            return Err(miette::miette!("simulated retrieve_blobs failure"));
        }

        let blobs = self.blobs.lock().unwrap();

        Ok(blob_digests
            .into_iter()
            .filter_map(|digest| {
                blobs.get(&digest).map(|bytes| BlobOutput {
                    content: BlobContent::Inline(bytes.clone()),
                    digest,
                })
            })
            .collect())
    }
}

fn digest(seed: char, size: i64) -> Digest {
    Digest {
        hash: ContentHash::from_hex(std::iter::repeat_n(seed, 64).collect::<String>()).unwrap(),
        size,
    }
}

fn manifest_with_file(blob: &Digest) -> TaskManifest {
    TaskManifest {
        files: vec![TaskManifestFile {
            bytes: Some(Bytes::from_static(b"output")),
            digest: Some(blob.clone()),
            path: "out/a.txt".into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Seed a backend so it "has" an entry: the manifest as a serializing backend
/// would persist it (no inline bytes) plus its referenced blob.
fn seed_backend(backend: &MemoryBackend, action: &Digest, blob: &Digest, bytes: &'static [u8]) {
    let mut manifest = manifest_with_file(blob);
    manifest.files[0].bytes = None;

    backend
        .manifests
        .lock()
        .unwrap()
        .insert(action.clone(), manifest);
    backend
        .blobs
        .lock()
        .unwrap()
        .insert(blob.clone(), Bytes::from_static(bytes));
}

mod storage {
    use super::*;

    #[tokio::test]
    async fn archive_then_load_and_hydrate_round_trip() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        storage
            .archive_task_manifest(&action, manifest_with_file(&blob))
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        // The persisted manifest comes back without inline bytes...
        let source = storage
            .load_task_manifest(&action)
            .await
            .unwrap()
            .expect("manifest was stored");
        assert!(!source.manifest.is_hydrated());

        // ...and hydration refills them from the stored blobs.
        let hydrated = storage
            .hydrate_task_manifest(&action, source)
            .await
            .unwrap()
            .expect("manifest was hydrated");
        assert!(hydrated.is_hydrated());
        assert_eq!(hydrated.files[0].bytes, Some(Bytes::from_static(b"output")));
    }

    #[tokio::test]
    async fn load_manifest_returns_none_when_absent() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        assert!(
            storage
                .load_task_manifest(&digest('a', 0))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn hydrate_copies_missing_blobs_from_secondary_to_primary() {
        let primary = MemoryBackend::new("primary");
        let secondary = MemoryBackend::new("secondary");

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"shared").unwrap();

        // Primary knows the manifest but is missing the blob; secondary has it.
        let mut manifest = manifest_with_file(&blob);
        manifest.files[0].bytes = None;
        primary
            .manifests
            .lock()
            .unwrap()
            .insert(action.clone(), manifest);

        let primary_blobs = Arc::clone(&primary.blobs);
        secondary
            .blobs
            .lock()
            .unwrap()
            .insert(blob.clone(), Bytes::from_static(b"shared"));

        let mut storage = create_storage();
        storage.add_local_backend(primary);
        storage.add_local_backend(secondary);

        let source = storage.load_task_manifest(&action).await.unwrap().unwrap();
        assert_eq!(source.backend.get_id().as_str(), "primary");

        let hydrated = storage
            .hydrate_task_manifest(&action, source)
            .await
            .unwrap()
            .expect("hydrated from secondary");
        assert!(hydrated.is_hydrated());

        // The missing blob was backfilled into the primary backend.
        assert!(primary_blobs.lock().unwrap().contains_key(&blob));
    }

    #[tokio::test]
    async fn archives_manifest_with_no_blobs() {
        // An exit-code-only manifest has no output files or stdio, so there are
        // no blobs to upload. It must still be archived, not skipped.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        let action = digest('a', 0);

        storage
            .archive_task_manifest(&action, TaskManifest::default())
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(
            storage.load_task_manifest(&action).await.unwrap().is_some(),
            "a blob-less manifest should still be stored"
        );
    }

    #[tokio::test]
    async fn archives_the_manifests_digest_source() {
        // The action digest addresses the fingerprint hash manifest. The caller
        // supplies it as the manifest's `digest_source`, and it must be uploaded
        // to the CAS alongside the outputs so an RE-compliant backend can
        // resolve the action result.
        let backend = MemoryBackend::new("mem");
        let blobs = Arc::clone(&backend.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let action = digest('a', 6);
        let output_blob = Digest::from_bytes(b"output").unwrap();

        let mut manifest = manifest_with_file(&output_blob);
        manifest.digest_source = Some(TaskManifestFile {
            bytes: Some(Bytes::from_static(b"action")),
            digest: Some(action.clone()),
            path: ".moon/cache/hashes/action.json".into(),
            ..Default::default()
        });

        storage
            .archive_task_manifest(&action, manifest)
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(
            blobs.lock().unwrap().contains_key(&action),
            "the action blob must be uploaded to the CAS"
        );
    }

    #[tokio::test]
    async fn archives_a_manifest_without_a_digest_source() {
        // Archiving without a fingerprint file on disk leaves `digest_source`
        // unset, which must still store the manifest rather than error.
        let backend = MemoryBackend::new("mem");
        let manifests = Arc::clone(&backend.manifests);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let action = digest('a', 6);
        let output_blob = Digest::from_bytes(b"output").unwrap();

        let manifest = manifest_with_file(&output_blob);
        assert!(manifest.digest_source.is_none());

        storage
            .archive_task_manifest(&action, manifest)
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(manifests.lock().unwrap().contains_key(&action));
    }

    #[tokio::test]
    async fn skips_manifest_when_blob_upload_fails() {
        // If a referenced blob fails to upload, the manifest must not be stored,
        // otherwise it would dangle pointing at a missing blob.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem").failing_store_blobs());

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        storage
            .archive_task_manifest(&action, manifest_with_file(&blob))
            .await
            .unwrap();
        // A failed upload must not surface as a program error.
        storage.wait_for_background_tasks().await.unwrap();

        assert!(
            storage.load_task_manifest(&action).await.unwrap().is_none(),
            "manifest must be skipped when a blob fails to upload"
        );
    }

    #[tokio::test]
    async fn skips_manifest_when_find_missing_fails() {
        // A failure in the existence pre-check aborts the store rather than
        // propagating, so the manifest is skipped and the run still succeeds.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem").failing_find_missing());

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        storage
            .archive_task_manifest(&action, manifest_with_file(&blob))
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(storage.load_task_manifest(&action).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn hydrate_returns_none_when_blob_unavailable() {
        // The manifest references a blob that no backend has, so it can't be
        // fully hydrated and must not be used.
        let backend = MemoryBackend::new("mem");
        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        let mut manifest = manifest_with_file(&blob);
        manifest.files[0].bytes = None;
        backend
            .manifests
            .lock()
            .unwrap()
            .insert(action.clone(), manifest);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let source = storage.load_task_manifest(&action).await.unwrap().unwrap();

        assert!(
            storage
                .hydrate_task_manifest(&action, source)
                .await
                .unwrap()
                .is_none(),
            "a partially hydrated manifest must yield None"
        );
    }

    #[tokio::test]
    async fn hydrate_returns_none_when_retrieve_fails() {
        // Even though the blob exists, a retrieval error must degrade to a cache
        // miss rather than failing the program.
        let backend = MemoryBackend::new("mem").failing_retrieve_blobs();
        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        let mut manifest = manifest_with_file(&blob);
        manifest.files[0].bytes = None;
        backend
            .manifests
            .lock()
            .unwrap()
            .insert(action.clone(), manifest);
        backend
            .blobs
            .lock()
            .unwrap()
            .insert(blob.clone(), Bytes::from_static(b"output"));

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let source = storage.load_task_manifest(&action).await.unwrap().unwrap();

        assert!(
            storage
                .hydrate_task_manifest(&action, source)
                .await
                .unwrap()
                .is_none()
        );
    }

    // ---- warm-on-remote-hit ----

    #[tokio::test]
    async fn remote_hit_warms_local_tier() {
        // The entry lives only in the remote; the local tier is cold. Hydrating
        // it should copy both the manifest and its blob into the local backend.
        let local = MemoryBackend::new("local");
        let remote = MemoryBackend::new("remote");

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        seed_backend(&remote, &action, &blob, b"output");
        let local_blobs = Arc::clone(&local.blobs);
        let local_manifests = Arc::clone(&local.manifests);

        let mut storage = create_storage();
        storage.add_local_backend(local);
        storage.add_remote_backend(remote);

        let source = storage.load_task_manifest(&action).await.unwrap().unwrap();
        assert!(source.remote, "entry must be served by the remote backend");

        let hydrated = storage
            .hydrate_task_manifest(&action, source)
            .await
            .unwrap()
            .expect("hydrated from remote");
        assert!(hydrated.is_hydrated());

        // Warming is queued in the background; drain it before asserting.
        storage.wait_for_background_tasks().await.unwrap();

        assert!(
            local_blobs.lock().unwrap().contains_key(&blob),
            "remote hit must warm the blob into the local tier"
        );
        assert!(
            local_manifests.lock().unwrap().contains_key(&action),
            "remote hit must warm the manifest into the local tier"
        );
    }

    #[tokio::test]
    async fn remote_hit_warms_every_local_backend() {
        // Warming targets the whole local tier, not just one backend, so it
        // scales to any number of configured local backends.
        let local_a = MemoryBackend::new("local-a");
        let local_b = MemoryBackend::new("local-b");
        let remote = MemoryBackend::new("remote");

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        seed_backend(&remote, &action, &blob, b"output");
        let a_blobs = Arc::clone(&local_a.blobs);
        let b_blobs = Arc::clone(&local_b.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(local_a);
        storage.add_local_backend(local_b);
        storage.add_remote_backend(remote);

        let source = storage.load_task_manifest(&action).await.unwrap().unwrap();
        storage
            .hydrate_task_manifest(&action, source)
            .await
            .unwrap()
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(a_blobs.lock().unwrap().contains_key(&blob));
        assert!(b_blobs.lock().unwrap().contains_key(&blob));
    }

    #[tokio::test]
    async fn local_hit_does_not_warm() {
        // A local hit needs no warming — the source already serves locally. A
        // second local backend stays untouched (cross-local warming is a
        // separate, deferred concern).
        let primary = MemoryBackend::new("primary");
        let secondary = MemoryBackend::new("secondary");

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        seed_backend(&primary, &action, &blob, b"output");
        let secondary_blobs = Arc::clone(&secondary.blobs);
        let secondary_manifests = Arc::clone(&secondary.manifests);

        let mut storage = create_storage();
        storage.add_local_backend(primary);
        storage.add_local_backend(secondary);

        let source = storage.load_task_manifest(&action).await.unwrap().unwrap();
        assert!(!source.remote, "entry must be served by a local backend");

        storage
            .hydrate_task_manifest(&action, source)
            .await
            .unwrap()
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(secondary_blobs.lock().unwrap().is_empty());
        assert!(secondary_manifests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn read_only_local_is_not_warmed() {
        // A read-only local backend can't be written, so warming must skip it
        // rather than erroring.
        let local = MemoryBackend::new("local").read_only();
        let remote = MemoryBackend::new("remote");

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        seed_backend(&remote, &action, &blob, b"output");
        let local_blobs = Arc::clone(&local.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(local);
        storage.add_remote_backend(remote);

        let source = storage.load_task_manifest(&action).await.unwrap().unwrap();
        storage
            .hydrate_task_manifest(&action, source)
            .await
            .unwrap()
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(
            local_blobs.lock().unwrap().is_empty(),
            "a read-only local backend must not be warmed"
        );
    }

    #[tokio::test]
    async fn warming_respects_excluded_local_tier() {
        // When the local tier is excluded from the active set, a remote hit must
        // warm nothing — warming honors the same options as reads.
        let local = MemoryBackend::new("local");
        let remote = MemoryBackend::new("remote");

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        seed_backend(&remote, &action, &blob, b"output");
        let local_blobs = Arc::clone(&local.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(local);
        storage.add_remote_backend(remote);

        let scoped = storage.with_options(StorageOptions {
            include_local: false,
            include_remote: true,
            ..Default::default()
        });

        let source = scoped.load_task_manifest(&action).await.unwrap().unwrap();
        assert!(source.remote);

        scoped
            .hydrate_task_manifest(&action, source)
            .await
            .unwrap()
            .unwrap();
        scoped.wait_for_background_tasks().await.unwrap();

        assert!(
            local_blobs.lock().unwrap().is_empty(),
            "warming must honor include_local = false"
        );
    }
}

mod hash_manifests {
    use super::*;

    #[derive(Serialize)]
    struct Fingerprint {
        command: &'static str,
        inputs: Vec<&'static str>,
    }

    fn fingerprint() -> Fingerprint {
        Fingerprint {
            command: "build",
            inputs: vec!["a.ts"],
        }
    }

    /// The bytes the hasher would serialize for the given contents, which is
    /// what a stored hash manifest blob must contain.
    fn envelope<T: Serialize>(contents: &[T]) -> Vec<u8> {
        format!(
            "[{}]",
            contents
                .iter()
                .map(|content| serde_json::to_string(content).unwrap())
                .collect::<Vec<_>>()
                .join(",")
        )
        .into_bytes()
    }

    #[tokio::test]
    async fn stores_a_blob_addressed_by_the_returned_digest() {
        // The returned digest is handed back to the caller as an action digest,
        // so it must actually address the bytes that were stored — otherwise the
        // manifest would point at a blob nobody can resolve.
        let backend = MemoryBackend::new("mem");
        let blobs = Arc::clone(&backend.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        let stored = blobs
            .lock()
            .unwrap()
            .get(&digest)
            .cloned()
            .expect("hash manifest was stored as a blob");

        assert_eq!(Digest::from_bytes(&stored).unwrap(), digest);
        assert_eq!(digest.size, stored.len() as i64);
    }

    #[tokio::test]
    async fn stores_the_hasher_envelope_not_the_raw_content() {
        // A hash manifest is the hasher's serialized form (contents wrapped in a
        // JSON array), not the bare content, since that's what the hash is
        // computed over.
        let backend = MemoryBackend::new("mem");
        let blobs = Arc::clone(&backend.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert_eq!(
            blobs.lock().unwrap().get(&digest).unwrap(),
            &Bytes::from(envelope(&[fingerprint()]))
        );
    }

    #[tokio::test]
    async fn stored_manifest_is_retrievable_through_storage() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        let blob = storage
            .retrieve_blob(digest.clone())
            .await
            .unwrap()
            .expect("blob is readable back out of storage");

        assert_eq!(blob.digest, digest);
        assert_eq!(
            blob.content.get_bytes().unwrap(),
            envelope(&[fingerprint()])
        );
    }

    #[tokio::test]
    async fn digests_are_content_addressed() {
        // Identical content must dedupe to one digest (and so one CAS entry),
        // while differing content must not collide.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        let one = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();
        let two = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();
        let other = storage
            .store_hash_manifest(
                "task",
                Fingerprint {
                    command: "test",
                    inputs: vec!["a.ts"],
                },
            )
            .await
            .unwrap();

        assert_eq!(one, two);
        assert_ne!(one, other);
    }

    #[tokio::test]
    async fn label_does_not_affect_the_digest() {
        // The label is a debugging aid only — it isn't hashed, so the same
        // content stored under different labels stays a single CAS entry.
        let backend = MemoryBackend::new("mem");
        let blobs = Arc::clone(&backend.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let one = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();
        let two = storage
            .store_hash_manifest("project", fingerprint())
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert_eq!(one, two);
        assert_eq!(blobs.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn stores_every_content_from_a_prebuilt_hasher() {
        // Callers that build up a hasher across many contents must get all of
        // them persisted, not just the first.
        let backend = MemoryBackend::new("mem");
        let blobs = Arc::clone(&backend.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let mut hasher = ContentHasher::new("task");
        hasher.hash_content(fingerprint()).unwrap();
        hasher.hash_content("extra").unwrap();

        let digest = storage
            .store_hash_manifest_with_hasher(hasher)
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        let stored = blobs.lock().unwrap().get(&digest).cloned().unwrap();

        assert_eq!(
            stored,
            Bytes::from(format!(
                "[{},\"extra\"]",
                serde_json::to_string(&fingerprint()).unwrap()
            ))
        );
        assert_eq!(Digest::from_bytes(&stored).unwrap(), digest);
    }

    #[tokio::test]
    async fn stores_a_hasher_with_no_content() {
        // The hasher only fills its serialization cache when the hash is
        // generated, so storing must hash before consuming the bytes. If that
        // order flipped, this would store zero bytes against a non-empty digest.
        let backend = MemoryBackend::new("mem");
        let blobs = Arc::clone(&backend.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let digest = storage
            .store_hash_manifest_with_hasher(ContentHasher::new("task"))
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        let stored = blobs.lock().unwrap().get(&digest).cloned().unwrap();

        assert_eq!(stored, Bytes::from_static(b"[]"));
        assert_eq!(digest.size, 2);
        assert_eq!(Digest::from_bytes(&stored).unwrap(), digest);
    }

    #[tokio::test]
    async fn stores_to_the_local_tier_only() {
        // A hash manifest describes the state of *this* machine (a toolchain it
        // installed, a file it synced), so it must never reach the remote tier
        // where another machine could read it as its own state.
        let local = MemoryBackend::new("local");
        let read_only = MemoryBackend::new("read-only").read_only();
        let remote = MemoryBackend::new("remote");

        let local_blobs = Arc::clone(&local.blobs);
        let read_only_blobs = Arc::clone(&read_only.blobs);
        let remote_blobs = Arc::clone(&remote.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(local);
        storage.add_local_backend(read_only);
        storage.add_remote_backend(remote);

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(local_blobs.lock().unwrap().contains_key(&digest));
        assert!(
            remote_blobs.lock().unwrap().is_empty(),
            "a hash manifest must not be uploaded to the remote tier"
        );
        assert!(
            read_only_blobs.lock().unwrap().is_empty(),
            "a read-only backend must not be written to"
        );
    }

    #[tokio::test]
    async fn stores_nothing_when_the_local_tier_is_scoped_out() {
        // Hash manifests only ever go local, so scoping the local tier out
        // leaves nowhere to put them. What matters is that the check agrees:
        // the same scope must report the hash as not stored, so a caller can
        // never mark work done that was never recorded.
        let local = MemoryBackend::new("local");
        let remote = MemoryBackend::new("remote");

        let local_blobs = Arc::clone(&local.blobs);
        let remote_blobs = Arc::clone(&remote.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(local);
        storage.add_remote_backend(remote);

        let scoped = storage.with_options(StorageOptions {
            include_local: false,
            include_remote: true,
            ..Default::default()
        });

        let digest = scoped
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        assert!(local_blobs.lock().unwrap().is_empty());
        assert!(
            remote_blobs.lock().unwrap().is_empty(),
            "a hash manifest must never reach the remote tier"
        );
        assert!(
            !scoped.has_hash_manifest(&digest).await,
            "the check must agree with the store"
        );
    }
}

mod hash_manifest_lookups {
    use super::*;

    /// Mirrors how `moon hash` resolves an abbreviated hash: ask each readable
    /// local backend, since remotes can't be enumerated.
    async fn find_by_prefix(storage: &Storage, prefix: &str) -> Vec<Digest> {
        let mut digests = vec![];

        for backend in storage.get_local_backends() {
            if backend.is_readable() {
                digests.extend(backend.find_blobs_by_prefix(prefix).await.unwrap());
            }
        }

        digests
    }

    #[derive(Serialize)]
    struct Fingerprint {
        command: &'static str,
    }

    fn fingerprint() -> Fingerprint {
        Fingerprint { command: "build" }
    }

    #[tokio::test]
    async fn reports_a_stored_manifest_as_present() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local"));

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        assert!(storage.has_hash_manifest(&digest).await);
    }

    #[tokio::test]
    async fn reports_an_unknown_manifest_as_absent() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local"));

        assert!(!storage.has_hash_manifest(&digest('a', 0)).await);
    }

    #[tokio::test]
    async fn reports_absent_when_there_is_no_local_tier() {
        // Without a local backend nothing can be stored, so the check must say
        // "not done" and let the caller redo the work rather than claiming it
        // was already handled.
        let storage = create_storage();

        assert!(!storage.has_hash_manifest(&digest('a', 0)).await);
    }

    #[tokio::test]
    async fn never_reports_present_from_a_remote_hit() {
        // A hash manifest records what *this* machine did. Another machine
        // having done it is not a reason to skip a local side effect.
        let remote = MemoryBackend::new("remote");
        let action = digest('a', 6);

        remote
            .blobs
            .lock()
            .unwrap()
            .insert(action.clone(), Bytes::from_static(b"[]"));

        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local"));
        storage.add_remote_backend(remote);

        assert!(!storage.has_hash_manifest(&action).await);
    }

    #[tokio::test]
    async fn treats_a_backend_failure_as_absent() {
        // Degrading to "re-run the work" is safe; claiming it was done when the
        // check never completed is not.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local").failing_find_missing());

        assert!(!storage.has_hash_manifest(&digest('a', 0)).await);
    }

    #[tokio::test]
    async fn loads_a_stored_manifests_bytes_back() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local"));

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        let bytes = storage
            .load_hash_manifest(&digest)
            .await
            .unwrap()
            .expect("the manifest reads back")
            .read_bytes()
            .unwrap();

        assert_eq!(Digest::from_bytes(&bytes).unwrap(), digest);
        assert_eq!(
            bytes,
            format!("[{}]", serde_json::to_string(&fingerprint()).unwrap()).into_bytes()
        );
    }

    #[tokio::test]
    async fn loading_an_unknown_manifest_is_none() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local"));

        assert!(
            storage
                .load_hash_manifest(&digest('a', 0))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn resolves_a_digest_from_a_partial_hash() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local"));

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        let prefix = &digest.hash.as_str()[0..8];
        let found = find_by_prefix(&storage, prefix).await;

        assert_eq!(found, vec![digest]);
    }

    #[tokio::test]
    async fn finds_nothing_for_an_unmatched_prefix() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local"));

        storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        assert!(find_by_prefix(&storage, "ffffffffff").await.is_empty());
    }

    #[tokio::test]
    async fn does_not_resolve_prefixes_against_the_remote_tier() {
        // Remotes can't be enumerated, so a prefix lookup must not appear to
        // work against one.
        let remote = MemoryBackend::new("remote");
        let action = digest('a', 6);

        remote
            .blobs
            .lock()
            .unwrap()
            .insert(action.clone(), Bytes::from_static(b"[]"));

        let mut storage = create_storage();
        storage.add_remote_backend(remote);

        assert!(
            find_by_prefix(&storage, &action.hash.as_str()[0..8])
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn reports_a_prefix_match_once_per_local_backend() {
        // The same manifest in two local backends comes back twice. Callers
        // that treat "more than one result" as an ambiguous abbreviation must
        // collapse duplicate digests first, or a second local backend would
        // make every lookup ambiguous.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local-a"));
        storage.add_local_backend(MemoryBackend::new("local-b"));

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        let found = find_by_prefix(&storage, &digest.hash.as_str()[0..8]).await;

        // Both backends hold it, so the caller sees it twice and has to treat
        // duplicates as one manifest rather than as an ambiguous abbreviation.
        assert_eq!(found, vec![digest.clone(), digest]);
    }
}
