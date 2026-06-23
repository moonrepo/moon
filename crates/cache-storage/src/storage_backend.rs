use crate::capabilities::CacheCapabilities;
use crate::helpers::{Partition, partition_into_batches};
use crate::manifest::Manifest;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobSource};
use moon_common::Id;
use moon_hash::Digest;
use moon_process::ProcessRegistry;
use rustc_hash::FxHashSet;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{trace, warn};

pub type BoxedStorageBackend = Arc<dyn StorageBackend>;

// API guidelines:
// - Retrieve methods take arguments as references.
// - Store/find methods take arguments as owned values.

#[async_trait]
pub trait StorageBackend: Send + Sync
where
    Self: 'static,
{
    fn get_capabilities(&self) -> &CacheCapabilities;
    fn get_id(&self) -> &Id;

    async fn load_capabilities(&self) -> miette::Result<CacheCapabilities>;

    /// Retrieve the manifest for the given digest if it exists, otherwise return `None`.
    /// This *does not* retrieve all the associated blobs for the manifest, only the manifest
    /// itself. Use `retrieve_blobs` to retrieve the blobs after retrieving the manifest.
    async fn retrieve_manifest(&self, digest: &Digest) -> miette::Result<Option<Manifest>>;

    /// Store the manifest for the given digest. This *does not* store the associated blobs for the
    /// manifest, only the manifest itself. Use `store_blobs` to store the blobs before the
    /// manifest, and ensure the manifest is only stored if all blobs are successfully stored.
    async fn store_manifest(&self, digest: Digest, manifest: Manifest) -> miette::Result<()>;

    //---------- FINDING BLOBS ----------//

    /// Find which blobs from the given list of blob digests are missing from the backend,
    /// and return the list of missing blob digests. This is used to determine which blobs need
    /// to be stored before storing a manifest. This method will automatically batch the requests
    /// based on the backend's capabilities, and will return the combined list of missing blob
    /// digests from all batches.
    async fn find_missing_blobs_batched(
        self: Arc<Self>,
        digest: Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>> {
        let cap = self.get_capabilities();
        let mut set = JoinSet::new();

        trace!(
            storage = self.get_id().as_str(),
            hash = digest.hash.as_str(),
            digests = blob_digests.len(),
            "Finding missing blobs",
        );

        for batch in partition_into_batches(
            blob_digests,
            cap.max_batch_total_size_bytes,
            // We are using the length of the digest itself, not the size of the blob,
            // because we are only checking for existence of the digest in the backend,
            // not the actual blob data!
            |digest| digest.len(),
        ) {
            let backend = Arc::clone(&self);

            set.spawn(Box::pin(async move {
                backend.find_missing_blobs(batch.items).await
            }));
        }

        let mut missing_digests = FxHashSet::default();

        while let Some(result) = set.join_next().await {
            missing_digests.extend(result.into_diagnostic()??);
        }

        if missing_digests.is_empty() {
            trace!(
                storage = self.get_id().as_str(),
                hash = digest.hash.as_str(),
                "No missing blobs, all exist in storage backend!",
            );
        } else {
            trace!(
                storage = self.get_id().as_str(),
                hash = digest.hash.as_str(),
                missing = missing_digests.len(),
                "Found missing blobs",
            );
        }

        Ok(missing_digests)
    }

    /// Determine which blobs from the given list of blob sources are missing from the backend,
    /// and return the list of missing blob digests. This is used to determine which blobs need
    /// to be uploaded before storing a manifest.
    async fn find_missing_blobs(
        &self,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<FxHashSet<Digest>>;

    //---------- STORING BLOBS ----------//

    /// Store the blobs from the given list of blob sources in batches. This method will
    /// automatically batch the requests based on the backend's capabilities, and will return
    /// `true` if all blobs were successfully stored, or `false` if any blobs failed to store.
    /// If any blobs fail to store, the backend should be considered in an inconsistent state,
    /// and the caller should handle the error accordingly.
    async fn store_blobs_batched(
        self: Arc<Self>,
        digest: Digest,
        mut blob_sources: Vec<BlobSource>,
    ) -> miette::Result<bool> {
        // Before we store blobs, we should ensure that they don't already exists in the backend
        let missing_digests = Arc::clone(&self)
            .find_missing_blobs_batched(digest.clone(), get_digests_from_sources(&blob_sources))
            .await?;

        if missing_digests.is_empty() {
            return Ok(true);
        }

        // Reduce the provided sources to only the missing digests
        blob_sources.retain(|source| missing_digests.contains(&source.digest));

        let cap = self.get_capabilities();
        let mut set = JoinSet::new();

        // Store the blobs in batches based on the max batch size
        for batch in
            partition_into_batches(blob_sources, cap.max_batch_total_size_bytes, |source| {
                source.digest.size as usize
            })
        {
            set.spawn(Box::pin(store_blobs_batch(
                Arc::clone(&self),
                digest.clone(),
                batch,
            )));
        }

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
                storage = self.get_id().as_str(),
                hash = digest.hash.as_str(),
                errors = ?upload_errors,
                "Failed to store blobs",
            );

            return Ok(false);
        }

        Ok(true)
    }

    /// Store the blobs from the given list of blob sources.
    async fn store_blobs(&self, blob_sources: Vec<BlobSource>) -> miette::Result<u16>;

    //---------- RECEIVING BLOBS ----------//

    async fn retrieve_blobs(&self, blob_digests: &[Digest]) -> miette::Result<Vec<Blob>>;
}

fn get_digests_from_sources(blob_sources: &[BlobSource]) -> Vec<Digest> {
    blob_sources
        .iter()
        .map(|source| source.digest.clone())
        .collect()
}

async fn store_blobs_batch<T: StorageBackend + ?Sized>(
    backend: Arc<T>,
    digest: Digest,
    batch: Partition<BlobSource>,
) -> miette::Result<bool> {
    let blob_count = batch.items.len();

    trace!(
        storage = backend.get_id().as_str(),
        hash = digest.hash.as_str(),
        blobs = blob_count,
        size = batch.size,
        "Storing blobs (batch {})",
        batch.key,
    );

    match backend.store_blobs(batch.items).await {
        Ok(count) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                blobs = count,
                missing = blob_count - (count as usize),
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
