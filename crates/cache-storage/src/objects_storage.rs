use crate::actions_storage::CacheContext;
use crate::storage_backend::{BoxedStorageBackend, StorageBackend};
use moon_blob::{Blob, BlobCleanStats, BlobContent, BlobInput, BlobOutput};
use moon_common::format_error_chain;
use moon_hash::{ContentHasher, Digest};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

#[derive(Debug)]
pub struct ObjectsStorage {
    backends: Vec<BoxedStorageBackend>,
    #[allow(dead_code)]
    context: CacheContext,
}

impl ObjectsStorage {
    pub fn new(context: CacheContext) -> Self {
        Self {
            backends: vec![],
            context,
        }
    }

    pub fn add_local_backend(&mut self, backend: impl StorageBackend + 'static) {
        self.backends.push(Arc::new(backend));
    }

    pub fn get_backends(&self) -> &[BoxedStorageBackend] {
        &self.backends
    }

    /// Garbage-collect the writable backends. A failure in one backend is logged
    /// and skipped rather than aborting the whole clean.
    pub async fn clean(&self, lifetime: Duration) -> miette::Result<BlobCleanStats> {
        let mut stats = BlobCleanStats::default();

        for backend in &self.backends {
            if !backend.is_writable() {
                continue;
            }

            match backend.gc(lifetime).await {
                Ok(backend_stats) => {
                    stats.blobs_removed += backend_stats.blobs_removed;
                    stats.bytes_saved += backend_stats.bytes_saved;
                }
                Err(error) => {
                    warn!(
                        storage = backend.get_id().as_str(),
                        error = format_error_chain(&error),
                        "Failed to garbage collect storage backend"
                    );
                }
            }
        }

        Ok(stats)
    }

    pub async fn retrieve_object(&self, digest: Digest) -> miette::Result<Option<BlobOutput>> {
        let mut results = self.retrieve_objects(vec![digest]).await?;

        if !results.is_empty() {
            return Ok(Some(results.remove(0)));
        }

        Ok(None)
    }

    pub async fn retrieve_objects(&self, digests: Vec<Digest>) -> miette::Result<Vec<BlobOutput>> {
        for backend in &self.backends {
            if !backend.is_readable() {
                continue;
            }

            let result = Arc::clone(backend)
                .retrieve_blobs_batched(Digest::default(), digests.clone())
                .await?;

            if !result.blobs.is_empty() {
                return Ok(result.blobs);
            }
        }

        Ok(vec![])
    }

    pub async fn store_object(&self, blob: BlobInput) -> miette::Result<()> {
        self.store_objects(vec![blob]).await
    }

    pub async fn store_objects(&self, blobs: Vec<BlobInput>) -> miette::Result<()> {
        for backend in &self.backends {
            if backend.is_writable() {
                Arc::clone(backend)
                    .store_blobs_batched(Digest::default(), blobs.clone())
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn store_hash_fingerprint<T: Serialize>(
        &self,
        label: &str,
        content: T,
    ) -> miette::Result<Digest> {
        let mut hasher = ContentHasher::new(label);
        hasher.hash_content(content)?;

        self.store_hash_fingerprint_with_hasher(hasher).await
    }

    pub async fn store_hash_fingerprint_with_hasher(
        &self,
        mut hasher: ContentHasher,
    ) -> miette::Result<Digest> {
        let digest = Digest::from_hasher(&mut hasher)?;

        debug!(
            label = hasher.label,
            "Storing hash fingerprint (local only)"
        );

        let data = hasher.into_bytes();

        for backend in &self.backends {
            if backend.is_writable() {
                Arc::clone(backend)
                    .store_blobs_batched(
                        digest.clone(),
                        vec![Blob::new(digest.clone(), data.clone()).into_input()],
                    )
                    .await?;
            }
        }

        Ok(digest)
    }

    pub async fn has_hash_fingerprint(&self, digest: &Digest) -> bool {
        for backend in &self.backends {
            if !backend.is_readable() {
                continue;
            }

            match backend.find_missing_blobs(vec![digest.to_owned()]).await {
                Ok(missing) => {
                    if missing.is_empty() {
                        return true;
                    }
                }
                Err(error) => {
                    warn!(
                        storage = backend.get_id().as_str(),
                        hash = digest.hash.as_str(),
                        error = format_error_chain(&error),
                        "Failed to check for a hash fingerprint, assuming it is missing"
                    );
                }
            }
        }

        false
    }

    pub async fn load_hash_fingerprint(
        &self,
        digest: &Digest,
    ) -> miette::Result<Option<BlobContent>> {
        Ok(self
            .retrieve_object(digest.to_owned())
            .await?
            .map(|blob| blob.content))
    }
}
