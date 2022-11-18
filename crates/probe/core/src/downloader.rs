use crate::errors::ProbeError;
use std::path::Path;

#[async_trait::async_trait]
pub trait Downloadable<T: Send + Sync>: Send + Sync {
    /// Returns an absolute file path to the downloaded file.
    /// This may not exist, as the path is composed ahead of time.
    /// This is typically ~/.prove/temp/<file>.
    fn get_download_path(&self) -> Result<&Path, ProbeError>;

    /// Determine whether the tool has already been downloaded.
    async fn is_downloaded(&self) -> Result<bool, ProbeError>;

    /// Downloads the tool (as an archive) from its distribution registry
    /// into the ~/.probe/temp folder.
    async fn download(&self, parent: &T) -> Result<(), ProbeError>;
}

#[async_trait::async_trait]
pub trait Verifiable<T: Send + Sync>: Send + Sync {
    /// If applicable, downloads all files necessary for verifying checksums.
    async fn download_checksum(&self, parent: &T) -> Result<(), ProbeError>;

    /// Verify the downloaded file using the checksum strategy for the tool.
    /// Common strategies are SHA256 and MD5.
    async fn verify_checksum(&self, file: &Path) -> Result<bool, ProbeError>;
}
