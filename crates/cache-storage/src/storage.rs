use crate::helpers::{Partition, partition_into_batches};
use crate::manifest::{Manifest, ManifestSource};
use crate::storage_backend::StorageBackend;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobContent, BlobSource, Bytes};
use moon_common::Id;
use moon_hash::Digest;
use moon_process::ProcessRegistry;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::task::{JoinHandle, JoinSet};
use tracing::{debug, trace, warn};

type BoxedStorageBackend = Arc<dyn StorageBackend>;

pub struct Storage {
    local_backends: Vec<BoxedStorageBackend>,
    remote_backends: Vec<BoxedStorageBackend>,
    background_tasks: Mutex<Vec<JoinHandle<miette::Result<()>>>>,
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

    pub async fn load_manifest(&self, digest: &Digest) -> miette::Result<Option<ManifestSource>> {
        for backend in self.get_backends() {
            if let Some(manifest) = backend.retrieve_manifest(digest).await? {
                return Ok(Some(ManifestSource {
                    manifest,
                    storage_id: backend.get_id().to_owned(),
                }));
            }
        }

        Ok(None)
    }

    pub async fn archive_manifest(
        &self,
        digest: &Digest,
        manifest: Manifest,
    ) -> miette::Result<()> {
        let mut background_tasks = self.background_tasks.lock().await;

        // Store the manifest in all backends in parallel, but if any fail,
        // continue storing the rest for failover/redundancy in the future
        for backend in self.get_backends() {
            background_tasks.push(tokio::spawn(Box::pin(archive_manifest_in_backend(
                Arc::clone(backend),
                digest.to_owned(),
                manifest.clone(),
            ))));
        }

        Ok(())
    }

    pub async fn hydrate_manifest(
        &self,
        digest: &Digest,
        mut manifest: Manifest,
        storage_id: Id,
    ) -> miette::Result<Option<Manifest>> {
        let mut backends = VecDeque::from_iter(self.get_backends());

        let original_backend = match backends
            .iter()
            .position(|backend| backend.get_id() == &storage_id)
        {
            Some(index) => backends.remove(index),
            None => None,
        };

        // Hydrate the manifest from the backend it was originally loaded from,
        // as that's the most likely to have all the blobs available
        if let Some(original_backend) = original_backend {
            hydrate_manifest_from_backend(original_backend, digest, &mut manifest).await?;
        }

        // If the original backend doesn't have all the blobs available,
        // we should attempt to hydrate from the other backends,
        // and also copy the missing blobs to the original backend
        while !manifest.is_hydrated()
            && let Some(backend) = backends.pop_front()
        {
            if let Some(original_backend) = original_backend {
                hydrate_manifest_from_backend_and_copy_to_original(
                    original_backend,
                    backend,
                    digest,
                    &mut manifest,
                )
                .await?;
            } else {
                hydrate_manifest_from_backend(backend, digest, &mut manifest).await?;
            }
        }

        // If the manifest is fully hydrated, return it, otherwise return None to
        // indicate it couldn't be fully hydrated, and should re-run
        if manifest.is_hydrated() {
            return Ok(Some(manifest));
        }

        Ok(None)
    }

    pub async fn wait_for_background_tasks(&self) -> miette::Result<()> {
        let background_tasks = {
            self.background_tasks
                .lock()
                .await
                .drain(0..)
                .collect::<Vec<_>>()
        };

        for handle in background_tasks {
            handle.await.into_diagnostic()??;
        }

        Ok(())
    }
}

async fn archive_manifest_in_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    mut manifest: Manifest,
) -> miette::Result<()> {
    let cap = backend.load_capabilities().await?;

    manifest.upload_started_at = Some(SystemTime::now());

    // Before we store the manifest, we should ensure all associated blobs are stored.
    // This ensures we don't end up with dangling manifests that reference missing blobs.
    let uploaded = store_blobs_in_backend(
        Arc::clone(&backend),
        digest.clone(),
        manifest.collect_blob_sources(),
    )
    .await?;

    manifest.upload_completed_at = Some(SystemTime::now());

    if !uploaded {
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
            "Successfully stored blobs but skipping cache manifest, as storage backend capabilities have it explicitly disabled",
        );
    }

    Ok(())
}

async fn store_blobs_in_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    blob_sources: Vec<BlobSource>,
) -> miette::Result<bool> {
    let cap = backend.load_capabilities().await?;
    let mut set = JoinSet::new();

    // Before we store the manifest, we should ensure all associated blobs are stored.
    // This ensures we don't end up with dangling manifests that reference missing blobs.
    let batches = partition_into_batches(blob_sources, cap.max_batch_total_size_bytes, |source| {
        source.digest.size as usize
    });
    let batch_total = batches.len();

    for (index, mut batch) in batches {
        batch.key = format!("{}:{batch_total}", index + 1);

        set.spawn(Box::pin(store_blobs_batch_in_backend(
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

        return Ok(false);
    }

    if !upload_errors.is_empty() {
        warn!(
            storage = backend.get_id().as_str(),
            hash = digest.hash.as_str(),
            errors = ?upload_errors,
            "Failed to store blobs for cache manifest",
        );

        return Ok(false);
    }

    Ok(true)
}

async fn store_blobs_batch_in_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    mut batch: Partition<BlobSource>,
) -> miette::Result<bool> {
    // Before we store blobs, we should ensure that they don't already exists in the backend
    let missing_digests = backend.find_missing_blobs(&batch.items).await?;

    if missing_digests.is_empty() {
        return Ok(true);
    }

    // Reduce the current batch to only the missing blobs
    batch
        .items
        .retain(|source| missing_digests.contains(&source.digest));

    // Calculate the true batch size for logging
    let size: i64 = batch.items.iter().map(|source| source.digest.size).sum();

    trace!(
        storage = backend.get_id().as_str(),
        hash = digest.hash.as_str(),
        blobs = batch.items.len(),
        size,
        "Storing blobs (batch {})",
        batch.key,
    );

    match backend.store_blobs(&batch.items).await {
        Ok(count) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                blobs = count,
                missing = batch.items.len() - (count as usize),
                "Stored blobs (batch {})",
                batch.key,
            );

            Ok(true)
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

async fn hydrate_manifest_from_backend(
    backend: &BoxedStorageBackend,
    digest: &Digest,
    manifest: &mut Manifest,
) -> miette::Result<FxHashMap<Digest, Bytes>> {
    let cap = backend.load_capabilities().await?;
    let mut set = JoinSet::new();

    // Before we hydrate the manifest, we should ensure all associated blobs are
    // retrieved in parallel based on unhydrated digests within the manifest
    let batches = partition_into_batches(
        manifest
            .collect_unhydrated_blob_digests()
            .into_iter()
            .cloned()
            .collect(),
        cap.max_batch_total_size_bytes,
        |digest| digest.size as usize,
    );
    let batch_total = batches.len();

    for (index, mut batch) in batches {
        batch.key = format!("{}:{batch_total}", index + 1);

        set.spawn(Box::pin(retrieve_blobs_batch_from_backend(
            Arc::clone(&backend),
            digest.clone(),
            batch,
        )));
    }

    // Retrieve each batch in parallel, and if any fail, continue retrieving the rest
    // as we'll attempt to retrieve the missing blobs from other backends
    let mut signal_receiver = ProcessRegistry::instance().receive_signal();
    let mut download_errors = vec![];
    let mut blobs_map = FxHashMap::default();
    let mut abort = false;

    while let Some(result) = set.join_next().await {
        if signal_receiver.try_recv().is_ok() {
            abort = true;
            break;
        }

        match result {
            Ok(Ok(blobs)) => {
                for blob in blobs {
                    blobs_map.insert(blob.digest, blob.bytes);
                }
            }
            Ok(Err(error)) => {
                download_errors.push(error.to_string());
            }
            Err(error) => {
                download_errors.push(error.to_string());
            }
        };
    }

    // If we received a shutdown signal, we should abort receiving the blobs
    if abort {
        set.shutdown().await;

        return Ok(blobs_map);
    }

    if !download_errors.is_empty() {
        debug!(
            storage = backend.get_id().as_str(),
            hash = digest.hash.as_str(),
            errors = ?download_errors,
            "Failed to retrieve blobs for cache manifest, will attempt to retrieve remaining from other storage backends",
        );
    }

    // Otherwise hydrate the manifest by copying the blobs into it
    manifest.hydrate(&blobs_map);

    Ok(blobs_map)
}

async fn hydrate_manifest_from_backend_and_copy_to_original(
    original_backend: &BoxedStorageBackend,
    backend: &BoxedStorageBackend,
    digest: &Digest,
    manifest: &mut Manifest,
) -> miette::Result<()> {
    // Collect the unhydrated blob digests from the manifest before hydrating,
    // so we can compare which are missing and attempt to copy them
    let unhydrated_digests = manifest
        .collect_unhydrated_blob_digests()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();

    let blobs_map = hydrate_manifest_from_backend(backend, digest, manifest).await?;

    // Loop through and create the blob sources for the missing blobs
    let mut blob_sources = vec![];

    for digest in unhydrated_digests {
        if let Some(bytes) = blobs_map.get(&digest) {
            blob_sources.push(BlobSource {
                content: BlobContent::Inline(bytes.to_owned()),
                digest,
            });
        }
    }

    if !blob_sources.is_empty() {
        store_blobs_in_backend(
            Arc::clone(original_backend),
            digest.to_owned(),
            blob_sources,
        )
        .await?;
    }

    Ok(())
}

async fn retrieve_blobs_batch_from_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    batch: Partition<Digest>,
) -> miette::Result<Vec<Blob>> {
    trace!(
        storage = backend.get_id().as_str(),
        hash = digest.hash.as_str(),
        blobs = batch.items.len(),
        "Retrieving blobs (batch {})",
        batch.key,
    );

    match backend.retrieve_blobs(&batch.items).await {
        Ok(blobs) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                blobs = blobs.len(),
                missing = batch.items.len() - blobs.len(),
                "Retrieved blobs (batch {})",
                batch.key,
            );

            Ok(blobs)
        }
        Err(error) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                error = error.to_string(),
                "Failed to retrieve blobs (batch {})",
                batch.key,
            );

            Err(error)
        }
    }
}
