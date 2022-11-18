mod downloader;
mod errors;
mod installer;
mod resolver;
mod verifier;

pub use async_trait::async_trait;
pub use downloader::*;
pub use errors::*;
pub use installer::*;
pub use lenient_semver::Version;
pub use resolver::*;
pub use verifier::*;

use std::path::PathBuf;

pub struct Probe {
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
}

#[async_trait::async_trait]
pub trait Tool<'tool>:
    Send + Sync + Resolvable<'tool> + Downloadable<'tool> + Verifiable<'tool>
{
    async fn setup(&mut self, parent: &Probe, initial_version: &str) -> Result<(), ProbeError> {
        self.resolve_version(initial_version, None).await?;

        // Download the archive
        let download_path = self.get_download_path(&parent.temp_dir)?;

        self.download(&download_path, None).await?;

        // Verify the archive
        let checksum_path = self.get_checksum_path(&parent.temp_dir)?;

        self.download_checksum(&checksum_path, None).await?;
        self.verify_checksum(&checksum_path, &download_path).await?;

        // TODO install

        Ok(())
    }
}
