use crate::bazel_compat::CacheCapabilities;
use crate::manifest::{Manifest, ManifestSource};
use async_trait::async_trait;
use moon_blob::BlobSource;
use moon_hash::Digest;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn load_capabilities(&self) -> miette::Result<CacheCapabilities>;

    /// Retrieve the manifest for the given digest if it exists, otherwise return `None`.
    /// This *does not* retrieve all the associated blobs for the manifest, only the manifest
    /// itself. Use `retrieve_blobs` to retrieve the blobs after retrieving the manifest.
    async fn retrieve_manifest(&self, digest: &Digest) -> miette::Result<Option<ManifestSource>>;

    /// Store the manifest for the given digest. This *does not* store the associated blobs for the
    /// manifest, only the manifest itself. Use `store_blobs` to store the blobs before the
    /// manifest, and ensure the manifest is only stored if all blobs are successfully stored.
    async fn store_manifest(&self, digest: &Digest, manifest: Manifest) -> miette::Result<()>;

    // async fn find_missing_blobs(&self, digest: &Digest, digests: &[Digest]) -> miette::Result<Vec<Digest>>;
    // async fn retrieve_blobs(&self, digest: &Digest, digests: &[Digest]) -> miette::Result<Vec<Blob>>;
    // async fn store_blobs(
    //     &self,
    //     digest: &Digest,
    //     sources: Vec<BlobSource>,
    // ) -> miette::Result<Vec<Digest>>;
}
