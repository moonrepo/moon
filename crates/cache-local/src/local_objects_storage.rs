use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{BlobCleanStats, BlobContent, BlobInput, BlobOutput};
use moon_cache_storage::{CacheCapabilities, CacheContext, StorageBackend, TaskManifest};
use moon_cas::CasStore;
use moon_common::Id;
use moon_hash::Digest;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::task::spawn_blocking;

#[derive(Debug)]
pub struct LocalObjectsStorage {
    id: Id,
    #[allow(dead_code)]
    context: CacheContext,

    // States
    capabilities: OnceLock<CacheCapabilities>,

    // Stores
    objects: Arc<CasStore>,
}

impl LocalObjectsStorage {
    pub fn new(context: CacheContext) -> miette::Result<Self> {
        Ok(Self {
            capabilities: OnceLock::new(),
            id: Id::raw("objects-cache"),
            objects: Arc::new(CasStore::new(
                context.cache_dir.join("objects"),
                Default::default(),
            )?),
            context,
        })
    }
}

#[async_trait]
impl StorageBackend for LocalObjectsStorage {
    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_capabilities(&self) -> &CacheCapabilities {
        self.capabilities.get_or_init(CacheCapabilities::default)
    }

    fn is_readable(&self) -> bool {
        true
    }

    fn is_writable(&self) -> bool {
        true
    }

    /// Nothing references objects (like hash manifests) the way task manifests
    /// reference blobs, so they're removed once older than the lifetime.
    async fn gc(&self, lifetime: Duration) -> miette::Result<BlobCleanStats> {
        self.objects.gc(lifetime).await
    }

    async fn retrieve_task_manifest(
        &self,
        _digest: Digest,
    ) -> miette::Result<Option<TaskManifest>> {
        unreachable!()
    }

    async fn store_task_manifest(
        &self,
        _digest: Digest,
        _manifest: TaskManifest,
    ) -> miette::Result<()> {
        unreachable!()
    }

    async fn find_missing_blobs(
        &self,
        mut blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Digest>> {
        let blobs = Arc::clone(&self.objects);

        spawn_blocking(move || {
            blob_digests.retain(|digest| !blobs.contains_object(digest));
            blob_digests
        })
        .await
        .into_diagnostic()
    }

    async fn retrieve_blobs(
        &self,
        blob_digests: Vec<Digest>,
        _stream: bool,
    ) -> miette::Result<Vec<BlobOutput>> {
        let blobs = Arc::clone(&self.objects);

        spawn_blocking(move || {
            Ok(blob_digests
                .into_iter()
                .filter_map(|digest| {
                    if blobs.contains_object(&digest) {
                        Some(BlobOutput {
                            content: BlobContent::File(blobs.object_path(&digest)),
                            digest,
                        })
                    } else {
                        None
                    }
                })
                .collect())
        })
        .await
        .into_diagnostic()?
    }

    async fn find_blobs_by_prefix(&self, prefix: &str) -> miette::Result<Vec<Digest>> {
        let blobs = Arc::clone(&self.objects);
        let prefix = prefix.to_owned();

        spawn_blocking(move || blobs.find_objects_by_prefix(&prefix))
            .await
            .into_diagnostic()?
    }

    async fn store_blobs(
        &self,
        blob_inputs: Vec<BlobInput>,
        _stream: bool,
    ) -> miette::Result<Vec<Digest>> {
        let blobs = Arc::clone(&self.objects);

        spawn_blocking(move || {
            let mut digests = vec![];

            for input in blob_inputs {
                let stored = match input.content {
                    BlobContent::File(abs_path) => blobs.write_file(&input.digest, &abs_path)?,
                    BlobContent::Inline(bytes) => blobs.write(&input.digest, &bytes)?,
                };

                if stored {
                    digests.push(input.digest);
                }
            }

            Ok(digests)
        })
        .await
        .into_diagnostic()?
    }
}
