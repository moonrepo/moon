use miette::IntoDiagnostic;
use moon_hash::Digest;
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::helpers::partition_into_batches;
use crate::manifest::{Manifest, ManifestSource};
use crate::storage_backend::StorageBackend;

type BoxedStorageBackend = Arc<dyn StorageBackend>;

pub struct Storage {
    local_backends: Vec<BoxedStorageBackend>,
    remote_backends: Vec<BoxedStorageBackend>,
}

impl Storage {
    pub fn add_local_backend(&mut self, backend: impl StorageBackend + 'static) {
        self.local_backends.push(Arc::new(backend));
    }

    pub fn add_remote_backend(&mut self, backend: impl StorageBackend + 'static) {
        self.remote_backends.push(Arc::new(backend));
    }

    pub fn get_backends(&self) -> Vec<&BoxedStorageBackend> {
        let mut list = vec![];
        list.extend(self.local_backends.iter());
        list.extend(self.remote_backends.iter());
        list
    }

    pub async fn retrieve_manifest(
        &self,
        digest: &Digest,
    ) -> miette::Result<Option<ManifestSource>> {
        for backend in self.get_backends() {
            if let Some(source) = backend.retrieve_manifest(digest).await? {
                return Ok(Some(source));
            }
        }

        Ok(None)
    }

    pub async fn store_manifest(&self, digest: &Digest, manifest: Manifest) -> miette::Result<()> {
        let mut set = JoinSet::new();

        for backend in self.get_backends() {
            let backend = Arc::clone(backend);
            let digest = digest.to_owned();
            let manifest = manifest.clone();

            set.spawn(async move { backend.store_manifest(&digest, manifest).await });
        }

        while let Some(result) = set.join_next().await {
            result.into_diagnostic()??;
        }

        Ok(())
    }
}

async fn save_manifest_to_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    manifest: Manifest,
) -> miette::Result<()> {
    let cap = backend.load_capabilities().await?;

    // Before we store the manifest, we should ensure all blobs are stored.
    // This ensures we don't end up with dangling manifests that reference missing blobs.
    let batches = partition_into_batches(
        manifest.collect_blob_sources(),
        cap.max_batch_total_size_bytes as usize,
        |source| source.digest.size as usize,
    );

    if !cap.store_manifests {
        // TODO warn
        return Ok(());
    }

    backend.store_manifest(&digest, manifest).await
}
