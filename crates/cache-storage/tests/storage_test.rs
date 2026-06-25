use async_trait::async_trait;
use moon_blob::{Blob, BlobContent, BlobSource, Bytes};
use moon_cache_storage::{CacheCapabilities, Manifest, ManifestFile, Storage, StorageBackend};
use moon_common::Id;
use moon_hash::{ContentHash, Digest};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::{Arc, Mutex};

/// In-memory backend with externally inspectable maps, so tests can both seed
/// state and assert on what was written back.
#[derive(Debug)]
struct MemoryBackend {
    id: Id,
    capabilities: CacheCapabilities,
    blobs: Arc<Mutex<FxHashMap<Digest, Bytes>>>,
    manifests: Arc<Mutex<FxHashMap<Digest, Manifest>>>,
}

impl MemoryBackend {
    fn new(id: &str) -> Self {
        Self {
            id: Id::raw(id),
            capabilities: CacheCapabilities::default(),
            blobs: Arc::new(Mutex::new(FxHashMap::default())),
            manifests: Arc::new(Mutex::new(FxHashMap::default())),
        }
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

    fn is_enabled(&self) -> bool {
        true
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

    async fn find_missing_blobs(
        &self,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>> {
        let blobs = self.blobs.lock().unwrap();

        Ok(blob_digests
            .into_iter()
            .filter(|digest| !blobs.contains_key(digest))
            .collect())
    }

    async fn store_blobs(
        &self,
        blob_sources: Vec<BlobSource>,
        _stream: bool,
    ) -> miette::Result<Vec<Digest>> {
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
    ) -> miette::Result<Vec<Blob>> {
        let blobs = self.blobs.lock().unwrap();

        Ok(blob_digests
            .into_iter()
            .filter_map(|digest| {
                blobs
                    .get(&digest)
                    .map(|bytes| Blob::new(digest, bytes.to_vec()))
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

mod storage {
    use super::*;

    #[tokio::test]
    async fn archive_then_load_and_hydrate_round_trip() {
        let mut storage = Storage::default();
        storage.add_local_backend(MemoryBackend::new("mem"));

        let action = digest('a', 0);
        let blob = Digest::from_bytes(b"output").unwrap();

        storage
            .archive_manifest(&action, manifest_with_file(&blob))
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
        let mut storage = Storage::default();
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

        let mut storage = Storage::default();
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
}
