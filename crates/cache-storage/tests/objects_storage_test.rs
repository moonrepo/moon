use async_trait::async_trait;
use moon_blob::{BlobContent, BlobInput, BlobOutput, Bytes};
use moon_cache_storage::{CacheCapabilities, CacheContext, ObjectsStorage, StorageBackend};
use moon_common::Id;
use moon_hash::{ContentHash, ContentHasher, Digest};
use moon_manifest::TaskManifest;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use starbase_utils::json::serde_json;
use std::path::Path;
use std::sync::{Arc, Mutex};

fn create_storage() -> ObjectsStorage {
    ObjectsStorage::new(CacheContext::new(Path::new("/moon-test")))
}

/// In-memory backend with an externally inspectable map, so tests can both
/// seed state and assert on what was written back.
#[derive(Debug)]
struct MemoryBackend {
    id: Id,
    capabilities: CacheCapabilities,
    blobs: Arc<Mutex<FxHashMap<Digest, Bytes>>>,

    // Failure injection for the degrade paths.
    fail_find_missing: bool,
    fail_store_blobs: bool,
    fail_retrieve_blobs: bool,

    // A read-only tier (e.g. a shared cache the user can't write) must never
    // be written to, and an unreadable one must never be read from.
    read_only: bool,
    unreadable: bool,
}

impl MemoryBackend {
    fn new(id: &str) -> Self {
        Self {
            id: Id::raw(id),
            capabilities: CacheCapabilities::default(),
            blobs: Arc::new(Mutex::new(FxHashMap::default())),
            fail_find_missing: false,
            fail_store_blobs: false,
            fail_retrieve_blobs: false,
            read_only: false,
            unreadable: false,
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

    fn unreadable(mut self) -> Self {
        self.unreadable = true;
        self
    }

    fn seed(&self, bytes: &'static [u8]) -> Digest {
        let digest = Digest::from_bytes(bytes).unwrap();

        self.blobs
            .lock()
            .unwrap()
            .insert(digest.clone(), Bytes::from_static(bytes));

        digest
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
        !self.unreadable
    }

    fn is_writable(&self) -> bool {
        !self.read_only
    }

    async fn retrieve_task_manifest(
        &self,
        _digest: Digest,
    ) -> miette::Result<Option<TaskManifest>> {
        unreachable!("objects storage never reads task manifests")
    }

    async fn store_task_manifest(
        &self,
        _digest: Digest,
        _manifest: TaskManifest,
    ) -> miette::Result<()> {
        unreachable!("objects storage never writes task manifests")
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

fn inline_input(bytes: &'static [u8]) -> BlobInput {
    BlobInput {
        content: BlobContent::Inline(Bytes::from_static(bytes)),
        digest: Digest::from_bytes(bytes).unwrap(),
    }
}

mod objects {
    use super::*;

    #[tokio::test]
    async fn stores_then_retrieves_an_object() {
        // Objects are written inline rather than queued in the background, so
        // they must be readable as soon as the store returns.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        let input = inline_input(b"object");
        let digest = input.digest.clone();

        storage.store_object(input).await.unwrap();

        let object = storage
            .retrieve_object(digest.clone())
            .await
            .unwrap()
            .expect("object is readable back out of storage");

        assert_eq!(object.digest, digest);
        assert_eq!(object.content.get_bytes().unwrap(), b"object");
    }

    #[tokio::test]
    async fn stores_then_retrieves_many_objects() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        let inputs = vec![
            inline_input(b"one"),
            inline_input(b"two"),
            inline_input(b"three"),
        ];
        let digests = inputs
            .iter()
            .map(|input| input.digest.clone())
            .collect::<Vec<_>>();

        storage.store_objects(inputs).await.unwrap();

        // Retrieval order across parallel batches isn't guaranteed.
        let retrieved = storage
            .retrieve_objects(digests.clone())
            .await
            .unwrap()
            .into_iter()
            .map(|object| object.digest)
            .collect::<FxHashSet<_>>();

        assert_eq!(retrieved, FxHashSet::from_iter(digests));
    }

    #[tokio::test]
    async fn retrieving_an_unknown_object_is_none() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        assert!(
            storage
                .retrieve_object(digest('a', 0))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn does_nothing_without_any_backends() {
        let storage = create_storage();
        let input = inline_input(b"object");
        let digest = input.digest.clone();

        storage.store_object(input).await.unwrap();

        assert!(storage.retrieve_object(digest).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn stores_to_every_writable_backend() {
        let a = MemoryBackend::new("a");
        let b = MemoryBackend::new("b");
        let read_only = MemoryBackend::new("read-only").read_only();

        let a_blobs = Arc::clone(&a.blobs);
        let b_blobs = Arc::clone(&b.blobs);
        let read_only_blobs = Arc::clone(&read_only.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(a);
        storage.add_local_backend(read_only);
        storage.add_local_backend(b);

        let input = inline_input(b"object");
        let digest = input.digest.clone();

        storage.store_object(input).await.unwrap();

        assert!(a_blobs.lock().unwrap().contains_key(&digest));
        assert!(b_blobs.lock().unwrap().contains_key(&digest));
        assert!(
            read_only_blobs.lock().unwrap().is_empty(),
            "a read-only backend must not be written to"
        );
    }

    #[tokio::test]
    async fn a_failing_backend_does_not_block_the_rest() {
        // A backend that fails to store is logged and skipped, so the backends
        // after it still receive the object.
        let healthy = MemoryBackend::new("healthy");
        let healthy_blobs = Arc::clone(&healthy.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("failing").failing_store_blobs());
        storage.add_local_backend(healthy);

        let input = inline_input(b"object");
        let digest = input.digest.clone();

        storage.store_object(input).await.unwrap();

        assert!(healthy_blobs.lock().unwrap().contains_key(&digest));
    }

    #[tokio::test]
    async fn retrieves_from_the_next_backend_on_a_miss() {
        let seeded = MemoryBackend::new("seeded");
        let digest = seeded.seed(b"object");

        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("empty"));
        storage.add_local_backend(seeded);

        let object = storage.retrieve_object(digest).await.unwrap().unwrap();

        assert_eq!(object.content.get_bytes().unwrap(), b"object");
    }

    #[tokio::test]
    async fn retrieves_from_the_next_backend_on_a_failure() {
        let failing = MemoryBackend::new("failing").failing_retrieve_blobs();
        let seeded = MemoryBackend::new("seeded");

        failing.seed(b"object");
        let digest = seeded.seed(b"object");

        let mut storage = create_storage();
        storage.add_local_backend(failing);
        storage.add_local_backend(seeded);

        let object = storage.retrieve_object(digest).await.unwrap().unwrap();

        assert_eq!(object.content.get_bytes().unwrap(), b"object");
    }

    #[tokio::test]
    async fn skips_unreadable_backends() {
        let unreadable = MemoryBackend::new("unreadable").unreadable();
        let digest = unreadable.seed(b"object");

        let mut storage = create_storage();
        storage.add_local_backend(unreadable);

        assert!(storage.retrieve_object(digest).await.unwrap().is_none());
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

        assert_eq!(
            blobs.lock().unwrap().get(&digest).unwrap(),
            &Bytes::from(envelope(&[fingerprint()]))
        );
    }

    #[tokio::test]
    async fn stored_manifest_is_retrievable_as_an_object() {
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("mem"));

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        let blob = storage
            .retrieve_object(digest.clone())
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

        let stored = blobs.lock().unwrap().get(&digest).cloned().unwrap();

        assert_eq!(stored, Bytes::from_static(b"[]"));
        assert_eq!(digest.size, 2);
        assert_eq!(Digest::from_bytes(&stored).unwrap(), digest);
    }

    #[tokio::test]
    async fn stores_to_every_writable_backend() {
        let a = MemoryBackend::new("a");
        let b = MemoryBackend::new("b");
        let read_only = MemoryBackend::new("read-only").read_only();

        let a_blobs = Arc::clone(&a.blobs);
        let b_blobs = Arc::clone(&b.blobs);
        let read_only_blobs = Arc::clone(&read_only.blobs);

        let mut storage = create_storage();
        storage.add_local_backend(a);
        storage.add_local_backend(read_only);
        storage.add_local_backend(b);

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        assert!(a_blobs.lock().unwrap().contains_key(&digest));
        assert!(b_blobs.lock().unwrap().contains_key(&digest));
        assert!(
            read_only_blobs.lock().unwrap().is_empty(),
            "a read-only backend must not be written to"
        );
    }
}

mod hash_manifest_lookups {
    use super::*;

    /// Mirrors how `moon hash` resolves an abbreviated hash: ask each readable
    /// backend, and collect every match.
    async fn find_by_prefix(storage: &ObjectsStorage, prefix: &str) -> Vec<Digest> {
        let mut digests = vec![];

        for backend in storage.get_backends() {
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
    async fn reports_absent_without_any_backends() {
        // Without a backend nothing can be stored, so the check must say
        // "not done" and let the caller redo the work rather than claiming it
        // was already handled.
        let storage = create_storage();

        assert!(!storage.has_hash_manifest(&digest('a', 0)).await);
    }

    #[tokio::test]
    async fn ignores_unreadable_backends() {
        let unreadable = MemoryBackend::new("unreadable").unreadable();
        let digest = unreadable.seed(b"[]");

        let mut storage = create_storage();
        storage.add_local_backend(unreadable);

        assert!(!storage.has_hash_manifest(&digest).await);
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
    async fn checks_the_next_backend_after_a_failure() {
        // A failing backend is logged and skipped, not treated as the final
        // answer, so a healthy backend after it can still report a hit.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("failing").failing_find_missing());
        storage.add_local_backend(MemoryBackend::new("healthy"));

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        assert!(storage.has_hash_manifest(&digest).await);
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
    async fn loads_from_the_next_backend_on_a_miss() {
        let seeded = MemoryBackend::new("seeded");
        let digest = seeded.seed(b"[]");

        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("empty"));
        storage.add_local_backend(seeded);

        let bytes = storage
            .load_hash_manifest(&digest)
            .await
            .unwrap()
            .expect("the manifest reads back")
            .read_bytes()
            .unwrap();

        assert_eq!(bytes, b"[]");
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
    async fn reports_a_prefix_match_once_per_backend() {
        // The same manifest in two backends comes back twice. Callers that
        // treat "more than one result" as an ambiguous abbreviation must
        // collapse duplicate digests first, or a second backend would make
        // every lookup ambiguous.
        let mut storage = create_storage();
        storage.add_local_backend(MemoryBackend::new("local-a"));
        storage.add_local_backend(MemoryBackend::new("local-b"));

        let digest = storage
            .store_hash_manifest("task", fingerprint())
            .await
            .unwrap();

        let found = find_by_prefix(&storage, &digest.hash.as_str()[0..8]).await;

        assert_eq!(found, vec![digest.clone(), digest]);
    }
}
