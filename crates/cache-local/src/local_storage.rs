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

pub struct LocalStorage {
    capabilities: CacheCapabilities,
    id: Id,
    blobs: CasStore,
    manifests: CasStore,
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
            blobs: CasStore::new(cache_dir.join("blobs"), &config.cas)?,
            manifests: CasStore::new(cache_dir.join("manifests"), &config.cas)?,
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
        if self.manifests.contains_object(&digest) {
            let blob = self.manifests.read(&digest)?;
            let manifest: Manifest = serde_json::from_slice(&blob).into_diagnostic()?;

            return Ok(Some(manifest));
        }

        Ok(None)
    }

    async fn store_manifest(&self, digest: Digest, manifest: Manifest) -> miette::Result<()> {
        if !self.manifests.contains_object(&digest) {
            let blob = Blob::from_data(manifest)?;

            self.manifests.write(&digest, &blob.bytes)?;
        }

        Ok(())
    }

    async fn find_missing_blobs(
        &self,
        mut blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>> {
        blob_digests.retain(|digest| !self.blobs.contains_object(digest));

        Ok(blob_digests.into_iter().collect())
    }

    async fn retrieve_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Blob>> {
        let mut blobs = vec![];

        for digest in blob_digests {
            blobs.push(self.blobs.retrieve_blob(&digest)?);
        }

        Ok(blobs)
    }

    async fn store_blobs(&self, blob_sources: Vec<BlobSource>) -> miette::Result<u16> {
        let mut count = 0;

        for source in blob_sources {
            match &source.content {
                BlobContent::File(rel_path) => {
                    let abs_path = rel_path.to_logical_path(&self.workspace_root);

                    if self.blobs.write_file(&source.digest, &abs_path)? {
                        count += 1;
                    }
                }
                BlobContent::Inline(bytes) => {
                    if self.blobs.write(&source.digest, bytes)? {
                        count += 1;
                    }
                }
            };
        }

        Ok(count)
    }
}
