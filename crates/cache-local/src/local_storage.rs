use std::path::{Path, PathBuf};
use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::Blob;
use moon_cache_storage::{Manifest, ManifestSource, StorageBackend};
use moon_cas::CasStore;
use moon_config::CacheConfig;
use moon_hash::Digest;

pub struct LocalStorage {
    dir: PathBuf,
    blobs: CasStore,
    manifests: CasStore,
}

impl LocalStorage {
    pub fn new(dir: impl AsRef<Path>, config: &CacheConfig) -> miette::Result<Self> {
        let dir = dir.as_ref();

        Ok(Self {
            blobs: CasStore::new(dir.join("blobs"), &config.cas)?,
            manifests: CasStore::new(dir.join("manifests"), &config.cas)?,
            dir: dir.to_path_buf(),
        })
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn retrieve_manifest(&self, digest: &Digest) -> miette::Result<Option<ManifestSource>> {
        if self.manifests.contains_object(&digest.hash) {
            let blob = self.manifests.read_bytes(&digest.hash)?;
            let manifest: Manifest = serde_json::from_slice(&blob).into_diagnostic()?;

            return Ok(Some(ManifestSource::Local(manifest)));
        }

        Ok(None)
    }

    async fn store_manifest(&self, digest: &Digest, manifest: Manifest) -> miette::Result<()> {
        if !self.manifests.contains_object(&digest.hash) {
            let blob = Blob::from_data(manifest)?;

            self.manifests.write(&digest.hash, &blob.bytes)?;
        }

        Ok(())
    }
}
