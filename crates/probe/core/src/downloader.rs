use crate::errors::ProbeError;
use crate::resolver::Resolvable;
use std::path::PathBuf;

#[async_trait::async_trait]
pub trait Downloadable<'tool, T: Send + Sync>: Send + Sync + Resolvable<'tool, T> {
    /// Returns an absolute file path to the downloaded file.
    /// This may not exist, as the path is composed ahead of time.
    /// This is typically ~/.prove/temp/<file>.
    fn get_download_path(&self, parent: &T) -> Result<PathBuf, ProbeError>;

    /// Determine whether the tool has already been downloaded.
    async fn is_downloaded(&self, parent: &T) -> Result<bool, ProbeError>;

    /// Download the tool (as an archive) from its distribution registry
    /// into the ~/.probe/temp folder.
    async fn download(&self, parent: &T) -> Result<(), ProbeError>;
}
