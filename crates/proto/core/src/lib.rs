mod describer;
mod downloader;
mod errors;
mod executor;
mod installer;
mod resolver;
mod verifier;

pub use async_trait::async_trait;
pub use describer::*;
pub use downloader::*;
pub use errors::*;
pub use executor::*;
pub use installer::*;
pub use lenient_semver::Version;
pub use resolver::*;
use std::fs;
use std::path::{Path, PathBuf};
pub use verifier::*;

pub struct Proto {
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
}

impl Proto {
    pub fn new(root: &Path) -> Self {
        Proto {
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
        }
    }
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
    + Executable<'tool>
{
    async fn before_setup(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    async fn setup(&mut self, initial_version: &str) -> Result<bool, ProtoError> {
        // Resolve a semantic version
        self.resolve_version(initial_version).await?;

        // Download the archive
        let download_path = self.get_download_path()?;

        self.download(&download_path, None).await?;

        // Verify the archive
        let checksum_path = self.get_checksum_path()?;

        self.download_checksum(&checksum_path, None).await?;
        self.verify_checksum(&checksum_path, &download_path).await?;

        // Install the tool
        let install_dir = self.get_install_dir()?;
        let installed = self.install(&install_dir, &download_path).await?;

        self.find_bin_path().await?;

        Ok(installed)
    }

    async fn is_setup(&mut self, initial_version: &str) -> Result<bool, ProtoError> {
        self.resolve_version(initial_version).await?;

        let install_dir = self.get_install_dir()?;

        if install_dir.exists() {
            self.find_bin_path().await?;

            return Ok(match self.get_bin_path() {
                Ok(bin) => bin.exists(),
                Err(_) => false,
            });
        }

        Ok(false)
    }

    async fn after_setup(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<(), ProtoError> {
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

    async fn before_teardown(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    async fn teardown(&mut self) -> Result<(), ProtoError> {
        self.cleanup().await?;

        let install_dir = self.get_install_dir()?;

        if install_dir.exists() {
            fs::remove_dir_all(&install_dir)
                .map_err(|e| ProtoError::Fs(install_dir, e.to_string()))?;
        }

        Ok(())
    }

    async fn after_teardown(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }
}
