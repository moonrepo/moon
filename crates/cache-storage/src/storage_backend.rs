use crate::capabilities::CacheCapabilities;
use crate::helpers::{Batch, create_batches};
use crate::manifest::Manifest;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{BlobInput, BlobOutput};
use moon_common::Id;
use moon_hash::Digest;
use moon_process::ProcessRegistry;
use rustc_hash::FxHashSet;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{trace, warn};

pub struct StoreResult {
    pub digests: Vec<Digest>,
    pub store_count: usize,
    pub stored_count: usize,
    pub missing_count: usize,
    pub success: bool,
}

pub struct RetrieveResult {
    pub blobs: Vec<BlobOutput>,
    pub retrieve_count: usize,
    pub retrieved_count: usize,
    pub success: bool,
}

pub type BoxedStorageBackend = Arc<dyn StorageBackend>;

#[async_trait]
pub trait StorageBackend: Debug + Send + Sync
where
    Self: 'static,
{
    fn get_id(&self) -> &Id;
    fn get_capabilities(&self) -> &CacheCapabilities;
    fn is_readable(&self) -> bool;
    fn is_writable(&self) -> bool;

    /// Connect to the storage backend, if necessary. This is called before any other methods are
    /// called, and can be used to establish a connection to a remote storage backend, or perform
    /// any other necessary setup. If the backend does not require a connection, this method can
    /// be a no-op.
    async fn connect(&self) -> miette::Result<()> {
        Ok(())
    }

    /// Retrieve the manifest for the given digest if it exists, otherwise return `None`.
    /// This *does not* retrieve all the associated blobs for the manifest, only the manifest
    /// itself. Use `retrieve_blobs` to retrieve the blobs after retrieving the manifest.
    async fn retrieve_manifest(&self, digest: Digest) -> miette::Result<Option<Manifest>>;

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
    ) -> miette::Result<Vec<Digest>> {
        let cap = self.get_capabilities();
        let mut set = JoinSet::new();

        trace!(
            storage = self.get_id().as_str(),
            hash = digest.hash.as_str(),
            digests = blob_digests.len(),
            "Finding missing blobs",
        );

        for batch in create_batches(
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

        let mut missing_digests = vec![];

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
    async fn find_missing_blobs(&self, blob_digests: Vec<Digest>) -> miette::Result<Vec<Digest>>;

    //---------- STORING BLOBS ----------//

    /// Store the blobs from the given list of blob sources in batches. This method will
    /// automatically batch the requests based on the backend's capabilities, and will return
    /// `true` if all blobs were successfully stored, or `false` if any blobs failed to store.
    /// If any blobs fail to store, the backend should be considered in an inconsistent state,
    /// and the caller should handle the error accordingly.
    async fn store_blobs_batched(
        self: Arc<Self>,
        digest: Digest,
        mut blob_inputs: Vec<BlobInput>,
    ) -> miette::Result<StoreResult> {
        // Outputs can share identical content, so the inputs may carry the same
        // digest more than once. Store each unique blob a single time: this keeps
        // the stored/missing counts consistent (the `success` check compares
        // them) and avoids redundant uploads when many files share a blob.
        let mut seen = FxHashSet::default();
        blob_inputs.retain(|input| seen.insert(input.digest.clone()));

        let mut result = StoreResult {
            digests: vec![],
            store_count: blob_inputs.len(),
            stored_count: 0,
            missing_count: 0,
            success: false,
        };

        // Before we store blobs, we should ensure that they don't already exists in the backend
        let missing_digests = match Arc::clone(&self)
            .find_missing_blobs_batched(digest.clone(), get_digests_from_inputs(&blob_inputs))
            .await
        {
            Ok(digests) => digests,
            Err(error) => {
                warn!(
                    storage = self.get_id().as_str(),
                    hash = digest.hash.as_str(),
                    error = error.to_string(),
                    "Failed to find missing blobs, aborting store operation",
                );

                return Ok(result);
            }
        };

        result.missing_count = missing_digests.len();

        if missing_digests.is_empty() {
            result.digests = get_digests_from_inputs(&blob_inputs);
            result.stored_count = result.store_count;
            result.success = true;

            return Ok(result);
        }

        // Reduce the provided sources to only the missing digests
        let missing_digests = FxHashSet::from_iter(missing_digests);
        blob_inputs.retain(|source| missing_digests.contains(&source.digest));

        let cap = self.get_capabilities();
        let mut set = JoinSet::new();

        // Store the blobs in batches based on the max batch size
        for batch in create_batches(blob_inputs, cap.max_batch_total_size_bytes, |source| {
            source.digest.size as usize
        }) {
            set.spawn(Box::pin(store_blobs_batch(
                Arc::clone(&self),
                digest.clone(),
                batch,
            )));
        }

        let mut signal_receiver = ProcessRegistry::instance().receive_signal();
        let mut upload_errors = vec![];
        let mut uploaded_digests = vec![];
        let mut abort = false;

        while let Some(result) = set.join_next().await {
            if signal_receiver.try_recv().is_ok() {
                abort = true;
                break;
            }

            match result {
                Ok(Ok(digests)) => {
                    uploaded_digests.extend(digests);
                }
                Ok(Err(error)) => {
                    upload_errors.push(error.to_string());
                }
                Err(error) => {
                    upload_errors.push(error.to_string());
                }
            };
        }

        result.stored_count = uploaded_digests.len();
        result.digests = uploaded_digests;

        // If we received a shutdown signal, we should abort storing the blobs
        if abort {
            set.shutdown().await;

            return Ok(result);
        }

        if upload_errors.is_empty() {
            result.success = result.stored_count == result.missing_count;
        } else {
            warn!(
                storage = self.get_id().as_str(),
                hash = digest.hash.as_str(),
                expected_count = result.missing_count,
                actual_count = result.stored_count,
                errors = ?upload_errors,
                "Failed to store blobs, will skip caching the manifest",
            );
        }

        Ok(result)
    }

    /// Store the blobs from the given list of blob sources.
    async fn store_blobs(
        &self,
        blob_inputs: Vec<BlobInput>,
        stream: bool,
    ) -> miette::Result<Vec<Digest>>;

    //---------- RECEIVING BLOBS ----------//

    /// Retrieve the blobs for the given list of blob digests in batches. This method will
    /// automatically batch the requests based on the backend's capabilities, and will return
    /// the combined list of retrieved blobs from all batches. If any blobs fail to retrieve,
    /// the backend should be considered in an inconsistent state, and the caller should handle
    /// the error accordingly.
    async fn retrieve_blobs_batched(
        self: Arc<Self>,
        digest: Digest,
        blob_digests: Vec<Digest>,
    ) -> miette::Result<RetrieveResult> {
        let mut result = RetrieveResult {
            blobs: vec![],
            retrieve_count: blob_digests.len(),
            retrieved_count: 0,
            success: false,
        };
        let cap = self.get_capabilities();
        let mut set = JoinSet::new();

        // Retrieve the blobs in batches based on the max batch size
        for batch in create_batches(blob_digests, cap.max_batch_total_size_bytes, |digest| {
            digest.size as usize
        }) {
            set.spawn(Box::pin(retrieve_blobs_batch(
                Arc::clone(&self),
                digest.clone(),
                batch,
            )));
        }

        let mut signal_receiver = ProcessRegistry::instance().receive_signal();
        let mut download_errors = vec![];
        let mut downloaded_blobs = vec![];
        let mut abort = false;

        while let Some(result) = set.join_next().await {
            if signal_receiver.try_recv().is_ok() {
                abort = true;
                break;
            }

            match result {
                Ok(Ok(batched_blobs)) => {
                    // Each backend verifies what it returns (gRPC and HTTP hash
                    // the bytes against the requested digest; local hands back
                    // CAS paths), so there's no second hash pass here.
                    downloaded_blobs.extend(batched_blobs);
                }
                Ok(Err(error)) => {
                    download_errors.push(error.to_string());
                }
                Err(error) => {
                    download_errors.push(error.to_string());
                }
            };
        }

        result.retrieved_count = downloaded_blobs.len();
        result.blobs = downloaded_blobs;

        // If we received a shutdown signal, we should abort receiving the blobs
        if abort {
            set.shutdown().await;

            return Ok(result);
        }

        if download_errors.is_empty() {
            result.success = result.retrieved_count == result.retrieve_count;
        } else {
            warn!(
                storage = self.get_id().as_str(),
                hash = digest.hash.as_str(),
                expected_count = result.retrieve_count,
                actual_count = result.retrieved_count,
                errors = ?download_errors,
                "Failed to retrieve blobs, will attempt to retrieve remaining from other storage backends",
            );
        }

        Ok(result)
    }

    /// Retrieve the blobs for the given list of blob digests.
    async fn retrieve_blobs(
        &self,
        blob_digests: Vec<Digest>,
        stream: bool,
    ) -> miette::Result<Vec<BlobOutput>>;
}

fn get_digests_from_inputs(blob_inputs: &[BlobInput]) -> Vec<Digest> {
    blob_inputs
        .iter()
        .map(|input| input.digest.clone())
        .collect()
}

async fn store_blobs_batch<T: StorageBackend + ?Sized>(
    backend: Arc<T>,
    digest: Digest,
    batch: Batch<BlobInput>,
) -> miette::Result<Vec<Digest>> {
    let blob_count = batch.items.len();

    trace!(
        storage = backend.get_id().as_str(),
        hash = digest.hash.as_str(),
        blobs = blob_count,
        size = batch.size,
        "Storing blobs (batch {}:{})",
        batch.index,
        batch.total,
    );

    match backend.store_blobs(batch.items, batch.stream).await {
        Ok(digests) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                blobs = digests.len(),
                missing = blob_count - digests.len(),
                "Stored blobs (batch {}:{})",
                batch.index,
                batch.total,
            );

            Ok(digests)
        }
        Err(error) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                error = error.to_string(),
                "Failed to store blobs (batch {}:{})",
                batch.index,
                batch.total,
            );

            Err(error)
        }
    }
}

async fn retrieve_blobs_batch<T: StorageBackend + ?Sized>(
    backend: Arc<T>,
    digest: Digest,
    batch: Batch<Digest>,
) -> miette::Result<Vec<BlobOutput>> {
    let blob_count = batch.items.len();

    trace!(
        storage = backend.get_id().as_str(),
        hash = digest.hash.as_str(),
        blobs = blob_count,
        size = batch.size,
        "Retrieving blobs (batch {}:{})",
        batch.index,
        batch.total,
    );

    match backend.retrieve_blobs(batch.items, batch.stream).await {
        Ok(blobs) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                blobs = blobs.len(),
                missing = blob_count - blobs.len(),
                "Retrieved blobs (batch {}:{})",
                batch.index,
                batch.total,
            );

            Ok(blobs)
        }
        Err(error) => {
            trace!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                error = error.to_string(),
                "Failed to retrieve blobs (batch {}:{})",
                batch.index,
                batch.total,
            );

            Err(error)
        }
    }
}
