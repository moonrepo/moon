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
       todo!("TODO")
    }

    async fn store_manifest(&self, digest: Digest, manifest: Manifest) -> miette::Result<()> {
        todo!("TODO")
    }

    async fn find_missing_blobs(
        &self,
        mut blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>> {
        todo!("TODO")
    }

    async fn retrieve_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Blob>> {
        todo!("TODO")
    }

    async fn store_blobs(&self, blob_sources: Vec<BlobSource>) -> miette::Result<u16> {
        todo!("TODO")
    }
}
