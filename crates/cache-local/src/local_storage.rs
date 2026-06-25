use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobContent, BlobSource};
use moon_cache_storage::{CacheCapabilities, CacheContext, Manifest, StorageBackend};
use moon_cas::CasStore;
use moon_common::Id;
use moon_hash::Digest;
use rustc_hash::FxHashSet;
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tokio::task::spawn_blocking;

#[derive(Debug)]
pub struct LocalStorage {
    id: Id,
    context: CacheContext,

    // States
    capabilities: OnceLock<CacheCapabilities>,

    // Stores
    blobs: Arc<CasStore>,
    manifests: Arc<CasStore>,
}

impl LocalStorage {
    pub fn new(
        context: CacheContext,
        cache_dir: impl AsRef<Path>,
        shared: bool,
    ) -> miette::Result<Self> {
        let cache_dir = cache_dir.as_ref();

        // Support for legacy cache directory structure
        let ac_dir = cache_dir.join("ac");
        let manifests_dir = cache_dir.join("manifests");

        let cas_dir = cache_dir.join("cas");
        let blobs_dir = cache_dir.join("blobs");

        if ac_dir.exists() {
            let _ = fs::rename(ac_dir, &manifests_dir);
        }

        if cas_dir.exists() {
            let _ = fs::rename(cas_dir, &blobs_dir);
        }

        Ok(Self {
            capabilities: OnceLock::new(),
            id: Id::raw(if shared {
                "shared-local-cache"
            } else {
                "local-cache"
            }),
            blobs: Arc::new(CasStore::new(blobs_dir, &context.cache_config.cas)?),
            manifests: Arc::new(CasStore::new(manifests_dir, &context.cache_config.cas)?),
            context,
        })
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_capabilities(&self) -> &CacheCapabilities {
        self.capabilities
            .get_or_init(|| CacheCapabilities::default())
    }

    fn is_enabled(&self) -> bool {
        true
    }

    async fn retrieve_manifest(&self, digest: Digest) -> miette::Result<Option<Manifest>> {
        let manifests = Arc::clone(&self.manifests);

        spawn_blocking(move || {
            if manifests.contains_object(&digest) {
                let blob = manifests.read(&digest)?;
                let manifest: Manifest = serde_json::from_slice(&blob).into_diagnostic()?;

                return Ok(Some(manifest));
            }

            Ok(None)
        })
        .await
        .into_diagnostic()?
    }

    async fn store_manifest(&self, digest: Digest, manifest: Manifest) -> miette::Result<()> {
        let manifests = Arc::clone(&self.manifests);

        spawn_blocking(move || {
            if !manifests.contains_object(&digest) {
                let blob = Blob::from_data(manifest)?;

                manifests.write(&digest, &blob.bytes)?;
            }

            Ok(())
        })
        .await
        .into_diagnostic()?
    }

    async fn find_missing_blobs(
        &self,
        mut blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>> {
        let blobs = Arc::clone(&self.blobs);

        spawn_blocking(move || {
            blob_digests.retain(|digest| !blobs.contains_object(digest));
            FxHashSet::from_iter(blob_digests)
        })
        .await
        .into_diagnostic()
    }

    async fn retrieve_blobs(
        &self,
        blob_digests: Vec<Digest>,
        _stream: bool,
    ) -> miette::Result<Vec<Blob>> {
        let blobs = Arc::clone(&self.blobs);

        spawn_blocking(move || {
            let mut result = Vec::with_capacity(blob_digests.len());

            for digest in blob_digests {
                result.push(blobs.retrieve_blob(&digest)?);
            }

            Ok(result)
        })
        .await
        .into_diagnostic()?
    }

    async fn store_blobs(
        &self,
        blob_sources: Vec<BlobSource>,
        _stream: bool,
    ) -> miette::Result<u16> {
        let blobs = Arc::clone(&self.blobs);
        let workspace_root = self.context.workspace_root.clone();

        spawn_blocking(move || {
            let mut count = 0;

            for source in blob_sources {
                let stored = match &source.content {
                    BlobContent::File(rel_path) => {
                        let abs_path = rel_path.to_logical_path(&workspace_root);

                        blobs.write_file(&source.digest, &abs_path)?
                    }
                    BlobContent::Inline(bytes) => blobs.write(&source.digest, bytes)?,
                };

                if stored {
                    count += 1;
                }
            }

            Ok(count)
        })
        .await
        .into_diagnostic()?
    }
}
