use crate::helpers::{Partition, partition_into_batches};
use crate::manifest::{Manifest, ManifestSource};
use crate::storage_backend::StorageBackend;
use miette::IntoDiagnostic;
use moon_blob::BlobSource;
use moon_hash::Digest;
use moon_process::ProcessRegistry;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::task::JoinSet;
use tracing::{debug, trace, warn};

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

    pub async fn save_manifest(&self, digest: &Digest, manifest: Manifest) -> miette::Result<()> {
        let mut set = JoinSet::new();

        for backend in self.get_backends() {
            set.spawn(Box::pin(store_manifest_in_backend(
                Arc::clone(backend),
                digest.to_owned(),
                manifest.clone(),
            )));
        }

        while let Some(result) = set.join_next().await {
            result.into_diagnostic()??;
        }

        Ok(())
    }
}

async fn store_manifest_in_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    mut manifest: Manifest,
) -> miette::Result<()> {
    let cap = backend.load_capabilities().await?;
    let mut set = JoinSet::new();

    // Before we store the manifest, we should ensure all associated blobs are stored.
    // This ensures we don't end up with dangling manifests that reference missing blobs.
    let batches = partition_into_batches(
        manifest.collect_blob_sources(),
        cap.max_batch_total_size_bytes,
        |source| source.digest.size as usize,
    );
    let batch_total = batches.len();

    manifest.upload_started_at = Some(SystemTime::now());

    for (index, mut batch) in batches {
        batch.key = format!("{}:{batch_total}", index + 1);

        set.spawn(Box::pin(store_blobs_in_backend(
            Arc::clone(&backend),
            digest.clone(),
            batch,
        )));
    }

    // Store each batch in parallel, and if any fail, continue storing the rest
    // but don't store the manifest and return early with a warning.
    let mut signal_receiver = ProcessRegistry::instance().receive_signal();
    let mut upload_errors = vec![];
    let mut abort = false;

    while let Some(result) = set.join_next().await {
        if signal_receiver.try_recv().is_ok() {
            abort = true;
            break;
        }

        match result {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                upload_errors.push(error.to_string());
            }
            Err(error) => {
                upload_errors.push(error.to_string());
            }
        };
    }

    // If we received a shutdown signal, we should abort storing the blobs
    if abort {
        set.shutdown().await;

        return Ok(());
    }

    manifest.upload_completed_at = Some(SystemTime::now());

    if !upload_errors.is_empty() {
        warn!(
            storage = backend.get_id().as_str(),
            hash = digest.hash.as_str(),
            errors = ?upload_errors,
            "Failed to store blobs for cache manifest",
        );

        return Ok(());
    }

    if cap.store_manifests {
        if let Err(error) = backend.store_manifest(&digest, manifest).await {
            warn!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                error = error.to_string(),
                "Failed to store cache manifest",
            );
        }
    } else {
        debug!(
            storage = backend.get_id().as_str(),
            hash = digest.hash.as_str(),
            "Stored blobs but skipping cache manifest, as storage backend capabilities have it disabled",
        );
    }

    Ok(())
}

async fn store_blobs_in_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    mut batch: Partition<BlobSource>,
) -> miette::Result<()> {
    // Before we store blobs, we should ensure that they don't already exists in the backend
    let missing_digests = backend.find_missing_blobs(&batch.items).await?;

    if missing_digests.is_empty() {
        return Ok(());
    }

    // Reduce the current batch to only the missing blobs
    batch
        .items
        .retain(|source| missing_digests.contains(&source.digest));

    // Recalculate the batch size after removing existing blobs
    batch.size = batch
        .items
        .iter()
        .map(|source| source.digest.size as usize)
        .sum();

    trace!(
        storage = backend.get_id().as_str(),
        hash = digest.hash.as_str(),
        blobs = batch.items.len(),
        size = batch.size,
        "Storing blobs (batch {})",
        batch.key,
    );

    match backend.store_blobs(&batch.items).await {
        Ok(_) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                "Stored blobs (batch {})",
                batch.key,
            );

            Ok(())
        }
        Err(error) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                error = error.to_string(),
                "Failed to store blobs (batch {})",
                batch.key,
            );

            Err(error)
        }
    }
}
