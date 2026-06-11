use moon_hash::Digest;
use crate::manifest::Manifest;

pub trait StorageBackend {
    async fn load_manifest(&self, digest: &Digest) -> miette::Result<Option<Manifest>>;
    async fn save_manifest(&self, digest: &Digest, manifest: &Manifest) -> miette::Result<()>;
}
