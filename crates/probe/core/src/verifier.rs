use crate::downloader::Downloadable;
use crate::errors::ProbeError;
use std::path::Path;

#[async_trait::async_trait]
pub trait Verifiable<'tool, T: Send + Sync>: Send + Sync + Downloadable<'tool, T> {
    /// If applicable, download all files necessary for verifying checksums.
    async fn download_checksum(&self, parent: &T) -> Result<(), ProbeError>;

    /// Verify the downloaded file using the checksum strategy for the tool.
    /// Common strategies are SHA256 and MD5.
    async fn verify_checksum(&self, file: &Path) -> Result<bool, ProbeError>;
}
