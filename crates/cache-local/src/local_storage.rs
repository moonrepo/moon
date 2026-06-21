use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobContent, BlobSource};
use moon_cache_storage::{CacheCapabilities, Manifest, ManifestSource, StorageBackend};
use moon_cas::CasStore;
use moon_common::Id;
use moon_config::CacheConfig;
use moon_hash::Digest;
use std::path::{Path, PathBuf};

pub struct LocalStorage {
    blobs: CasStore,
    id: Id,
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
            id: Id::raw("local-cache"),
            blobs: CasStore::new(cache_dir.join("blobs"), &config.cas)?,
            manifests: CasStore::new(cache_dir.join("manifests"), &config.cas)?,
            workspace_root: workspace_root.to_path_buf(),
        })
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    fn get_id(&self) -> &Id {
        &self.id
    }

    async fn load_capabilities(&self) -> miette::Result<CacheCapabilities> {
        Ok(CacheCapabilities::default())
    }

    async fn retrieve_manifest(&self, digest: &Digest) -> miette::Result<Option<ManifestSource>> {
        if self.manifests.contains_object(digest) {
            let blob = self.manifests.read_bytes(digest)?;
            let manifest: Manifest = serde_json::from_slice(&blob).into_diagnostic()?;

            return Ok(Some(ManifestSource::Local(manifest)));
        }

        Ok(None)
    }

    async fn store_manifest(&self, digest: &Digest, manifest: Manifest) -> miette::Result<()> {
        if !self.manifests.contains_object(digest) {
            let blob = Blob::from_data(manifest)?;

            self.manifests.write(digest, &blob.bytes)?;
        }

        Ok(())
    }

    async fn find_missing_blobs(&self, blob_sources: &[BlobSource]) -> miette::Result<Vec<Digest>> {
        let mut missing_digests = vec![];

        for source in blob_sources {
            if !self.blobs.contains_object(&source.digest) {
                missing_digests.push(source.digest.clone());
            }
        }

        Ok(missing_digests)
    }

    async fn store_blobs(&self, blob_sources: &[BlobSource]) -> miette::Result<()> {
        for source in blob_sources {
            match &source.content {
                BlobContent::Inline(bytes) => {
                    self.blobs.write(&source.digest, bytes)?;
                }
                BlobContent::File(rel_path) => {
                    let abs_path = rel_path.to_logical_path(&self.workspace_root);

                    // TODO reuse existing digest
                    self.blobs.write_path(&abs_path)?;
                }
            };
        }

        Ok(())
    }
}
