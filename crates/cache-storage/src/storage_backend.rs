use crate::capabilities::CacheCapabilities;
use crate::manifest::Manifest;
use async_trait::async_trait;
use moon_blob::{Blob, BlobSource};
use moon_common::Id;
use moon_hash::Digest;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    fn get_id(&self) -> &Id;

    async fn load_capabilities(&self) -> miette::Result<CacheCapabilities>;

    /// Retrieve the manifest for the given digest if it exists, otherwise return `None`.
    /// This *does not* retrieve all the associated blobs for the manifest, only the manifest
    /// itself. Use `retrieve_blobs` to retrieve the blobs after retrieving the manifest.
    async fn retrieve_manifest(&self, digest: &Digest) -> miette::Result<Option<Manifest>>;

    /// Store the manifest for the given digest. This *does not* store the associated blobs for the
    /// manifest, only the manifest itself. Use `store_blobs` to store the blobs before the
    /// manifest, and ensure the manifest is only stored if all blobs are successfully stored.
    async fn store_manifest(&self, digest: &Digest, manifest: Manifest) -> miette::Result<()>;

    /// Determine which blobs from the given list of blob sources are missing from the backend,
    /// and return the list of missing blob digests. This is used to determine which blobs need
    /// to be uploaded before storing a manifest.
    async fn find_missing_blobs(&self, blob_sources: &[BlobSource]) -> miette::Result<Vec<Digest>>;

    async fn retrieve_blobs(&self, blob_digests: &[Digest]) -> miette::Result<Vec<Blob>>;

    /// Store the blobs from the given list of blob sources. This should only be called after
    /// `find_missing_blobs` is called to ensure only missing blobs are stored, and should be
    /// called before `store_manifest` to ensure the manifest is only stored if all blobs are
    /// successfully stored.
    async fn store_blobs(&self, blob_sources: &[BlobSource]) -> miette::Result<u16>;
}
