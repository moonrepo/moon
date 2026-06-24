use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobContent, BlobSource};
use moon_cache_storage::{CacheCapabilities, Manifest, StorageBackend};
use moon_cas::CasStore;
use moon_common::Id;
use moon_config::CacheConfig;
use moon_hash::Digest;
use rustc_hash::FxHashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::spawn_blocking;

pub struct LocalStorage {
    capabilities: CacheCapabilities,
    id: Id,
    blobs: Arc<CasStore>,
    manifests: Arc<CasStore>,
    workspace_root: PathBuf,
}

impl LocalStorage {
    pub fn new(
        workspace_root: impl AsRef<Path>,
        cache_dir: impl AsRef<Path>,
        config: &CacheConfig,
    ) -> miette::Result<Self> {
        let workspace_root = workspace_root.as_ref();
        let cache_dir = cache_dir.as_ref();

        Ok(Self {
            capabilities: CacheCapabilities::default(),
            id: Id::raw("local-cache"),
            blobs: Arc::new(CasStore::new(cache_dir.join("blobs"), &config.cas)?),
            manifests: Arc::new(CasStore::new(cache_dir.join("manifests"), &config.cas)?),
            workspace_root: workspace_root.to_path_buf(),
        })
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    fn get_capabilities(&self) -> &CacheCapabilities {
        &self.capabilities
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    async fn load_capabilities(&self) -> miette::Result<CacheCapabilities> {
        Ok(CacheCapabilities::default())
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

    async fn retrieve_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Blob>> {
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

    async fn store_blobs(&self, blob_sources: Vec<BlobSource>) -> miette::Result<u16> {
        let blobs = Arc::clone(&self.blobs);
        let workspace_root = self.workspace_root.clone();

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
