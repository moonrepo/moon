mod describer;
mod downloader;
mod errors;
mod installer;
mod resolver;
mod verifier;

pub use async_trait::async_trait;
pub use describer::*;
pub use downloader::*;
pub use errors::*;
pub use installer::*;
pub use lenient_semver::Version;
pub use resolver::*;
use std::fs;
use std::path::PathBuf;
pub use verifier::*;

pub struct Probe {
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
}

#[async_trait::async_trait]
pub trait Tool<'tool>:
    Send
    + Sync
    + Describable<'tool>
    + Resolvable<'tool>
    + Downloadable<'tool>
    + Verifiable<'tool>
    + Installable<'tool>
{
    async fn before_setup(&mut self) -> Result<(), ProbeError> {
        Ok(())
    }

    async fn setup(&mut self, initial_version: &str) -> Result<(), ProbeError> {
        // Resolve a semantic version
        self.resolve_version(initial_version, None).await?;

        // Download the archive
        let download_path = self.get_download_path()?;

        self.download(&download_path, None).await?;

        // Verify the archive
        let checksum_path = self.get_checksum_path()?;

        self.download_checksum(&checksum_path, None).await?;
        self.verify_checksum(&checksum_path, &download_path).await?;

        // Install the tool
        let install_dir = self.get_install_dir()?;

        self.install(&install_dir, &download_path).await?;

        // Cleanup temp files
        self.cleanup().await?;

        Ok(())
    }

    async fn after_setup(&mut self) -> Result<(), ProbeError> {
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<(), ProbeError> {
        let download_path = self.get_download_path()?;
        let checksum_path = self.get_checksum_path()?;

        if download_path.exists() {
            let _ = fs::remove_file(download_path);
        }

        if checksum_path.exists() {
            let _ = fs::remove_file(checksum_path);
        }

        Ok(())
    }

    async fn before_teardown(&mut self) -> Result<(), ProbeError> {
        Ok(())
    }

    async fn teardown(&mut self) -> Result<(), ProbeError> {
        self.cleanup().await?;

        Ok(())
    }

    async fn after_teardown(&mut self) -> Result<(), ProbeError> {
        Ok(())
    }
}
