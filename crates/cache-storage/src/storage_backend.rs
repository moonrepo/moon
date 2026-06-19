use async_trait::async_trait;
use bazel_remote_apis::build::bazel::remote::execution::v2::{CacheCapabilities};
use moon_blob::Blob;
use moon_hash::Digest;
use crate::manifest::{Manifest, ManifestSource};

pub struct BlobSource;


#[async_trait]
pub trait StorageBackend: Send + Sync {
    // async fn load_capabilities(&self) -> miette::Result<CacheCapabilities>;

    async fn retrieve_manifest(&self, digest: &Digest) -> miette::Result<Option<ManifestSource>>;
    async fn store_manifest(&self, digest: &Digest, manifest: Manifest) -> miette::Result<()>;

    // async fn find_missing_blobs(&self, digest: &Digest, digests: &[Digest]) -> miette::Result<Vec<Digest>>;
    // async fn retrieve_blobs(&self, digest: &Digest, digests: &[Digest]) -> miette::Result<Vec<Blob>>;
    // async fn store_blobs(&self, digest: &Digest, sources: &[BlobSource]) -> miette::Result<Vec<Digest>>;
}
