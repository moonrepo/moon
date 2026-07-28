use async_trait::async_trait;
use moon_blob::{BlobContent, BlobInput, BlobOutput, Bytes};
use moon_cache_storage::{
    CacheCapabilities, CacheContext, Manifest, ManifestFile, Storage, StorageBackend,
    StorageOptions,
};
use moon_common::Id;
use moon_config::{CacheConfig, RemoteConfig};
use moon_hash::{ContentHash, Digest};
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn create_storage() -> Storage {
    Storage::new(CacheContext {
        cache_dir: PathBuf::from("/moon-test/.moon/cache"),
        cache_config: Arc::new(CacheConfig::default()),
        config_dir: PathBuf::from("/moon-test/.moon"),
        remote_config: Arc::new(RemoteConfig::default()),
        remote_debug: false,
        workspace_root: PathBuf::from("/moon-test"),
    })
}

/// In-memory backend with externally inspectable maps, so tests can both seed
/// state and assert on what was written back.
#[derive(Debug)]
struct MemoryBackend {
    id: Id,
    capabilities: CacheCapabilities,
    blobs: Arc<Mutex<FxHashMap<Digest, Bytes>>>,
    manifests: Arc<Mutex<FxHashMap<Digest, Manifest>>>,

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

    async fn retrieve_manifest(&self, digest: Digest) -> miette::Result<Option<Manifest>> {
        Ok(self.manifests.lock().unwrap().get(&digest).cloned())
    }

    async fn store_manifest(&self, digest: Digest, mut manifest: Manifest) -> miette::Result<()> {
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

fn manifest_with_file(blob: &Digest) -> Manifest {
    Manifest {
        files: vec![ManifestFile {
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
            .archive_manifest(&action, manifest_with_file(&blob), None)
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        // The persisted manifest comes back without inline bytes...
        let source = storage
            .load_manifest(&action)
            .await
            .unwrap()
            .expect("manifest was stored");
        assert!(!source.manifest.is_hydrated());

        // ...and hydration refills them from the stored blobs.
        let hydrated = storage
            .hydrate_manifest(&action, source)
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
                .load_manifest(&digest('a', 0))
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

        let source = storage.load_manifest(&action).await.unwrap().unwrap();
        assert_eq!(source.backend.get_id().as_str(), "primary");

        let hydrated = storage
            .hydrate_manifest(&action, source)
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
            .archive_manifest(&action, Manifest::default(), None)
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(
            storage.load_manifest(&action).await.unwrap().is_some(),
            "a blob-less manifest should still be stored"
        );
    }

    #[tokio::test]
    async fn archives_the_provided_action_blob() {
        // The action digest addresses the fingerprint hash manifest. When the
        // caller supplies it as a blob, it must be uploaded to the CAS alongside
        // the outputs so an RE-compliant backend can resolve the action result.
        let backend = MemoryBackend::new("mem");
        let blobs = Arc::clone(&backend.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(backend);

        let action = digest('a', 6);
        let output_blob = Digest::from_bytes(b"output").unwrap();
        let action_blob = BlobInput {
            content: BlobContent::Inline(Bytes::from_static(b"action")),
            digest: action.clone(),
        };

        storage
            .archive_manifest(&action, manifest_with_file(&output_blob), Some(action_blob))
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(
            blobs.lock().unwrap().contains_key(&action),
            "the action blob must be uploaded to the CAS"
        );
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
            .archive_manifest(&action, manifest_with_file(&blob), None)
            .await
            .unwrap();
        // A failed upload must not surface as a program error.
        storage.wait_for_background_tasks().await.unwrap();

        assert!(
            storage.load_manifest(&action).await.unwrap().is_none(),
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
            .archive_manifest(&action, manifest_with_file(&blob), None)
            .await
            .unwrap();
        storage.wait_for_background_tasks().await.unwrap();

        assert!(storage.load_manifest(&action).await.unwrap().is_none());
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

        let source = storage.load_manifest(&action).await.unwrap().unwrap();

        assert!(
            storage
                .hydrate_manifest(&action, source)
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

        let source = storage.load_manifest(&action).await.unwrap().unwrap();

        assert!(
            storage
                .hydrate_manifest(&action, source)
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

        let source = storage.load_manifest(&action).await.unwrap().unwrap();
        assert!(source.remote, "entry must be served by the remote backend");

        let hydrated = storage
            .hydrate_manifest(&action, source)
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

        let source = storage.load_manifest(&action).await.unwrap().unwrap();
        storage
            .hydrate_manifest(&action, source)
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

        let source = storage.load_manifest(&action).await.unwrap().unwrap();
        assert!(!source.remote, "entry must be served by a local backend");

        storage
            .hydrate_manifest(&action, source)
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

        let source = storage.load_manifest(&action).await.unwrap().unwrap();
        storage
            .hydrate_manifest(&action, source)
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

        let source = scoped.load_manifest(&action).await.unwrap().unwrap();
        assert!(source.remote);

        scoped
            .hydrate_manifest(&action, source)
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
