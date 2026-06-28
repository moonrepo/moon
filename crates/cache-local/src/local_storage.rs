use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobContent, BlobInput, BlobOutput};
use moon_cache_storage::{CacheCapabilities, CacheContext, Manifest, StorageBackend};
use moon_cas::CasStore;
use moon_common::Id;
use moon_hash::Digest;
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tokio::task::spawn_blocking;

#[derive(Debug)]
pub struct LocalStorage {
    id: Id,
    #[allow(dead_code)]
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

        let cas_config = context.cache_config.cas.clone();

        Ok(Self {
            capabilities: OnceLock::new(),
            id: Id::raw(if shared {
                "shared-local-cache"
            } else {
                "local-cache"
            }),
            manifests: Arc::new(CasStore::new(manifests_dir, {
                let mut config = cas_config.clone();
                // Our manifest hashes do not align with their contents,
                // so avoid verifying integrity for now!
                config.verify_integrity = false;
                config
            })?),
            blobs: Arc::new(CasStore::new(blobs_dir, cas_config)?),
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
        self.capabilities.get_or_init(CacheCapabilities::default)
    }

    fn is_readable(&self) -> bool {
        true
    }

    fn is_writable(&self) -> bool {
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
    ) -> miette::Result<Vec<Digest>> {
        let blobs = Arc::clone(&self.blobs);

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
        let blobs = Arc::clone(&self.blobs);

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

    async fn store_blobs(
        &self,
        blob_inputs: Vec<BlobInput>,
        _stream: bool,
    ) -> miette::Result<Vec<Digest>> {
        let blobs = Arc::clone(&self.blobs);

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
