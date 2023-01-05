mod describer;
mod detector;
mod downloader;
mod executor;
mod helpers;
mod installer;
mod resolver;
mod shimmer;
mod verifier;

pub use async_trait::async_trait;
pub use describer::*;
pub use detector::*;
pub use downloader::*;
pub use executor::*;
pub use helpers::*;
pub use installer::*;
pub use lenient_semver::Version;
pub use proto_error::ProtoError;
pub use resolver::*;
pub use shimmer::*;
pub use verifier::*;

use std::fs;
use std::path::{Path, PathBuf};

pub struct Proto {
    pub shims_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
}

impl Proto {
    pub fn new() -> Result<Self, ProtoError> {
        let root = get_root()?;

        Ok(Proto {
            shims_dir: root.join("shims"),
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
        })
    }

    pub fn from(root: &Path) -> Self {
        Proto {
            shims_dir: root.join("shims"),
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
    + Detector<'tool>
    + Resolvable<'tool>
    + Downloadable<'tool>
    + Verifiable<'tool>
    + Installable<'tool>
    + Executable<'tool>
    + Shimable<'tool>
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

        // Create shims after paths are found
        self.create_shims().await?;

        Ok(installed)
    }

    async fn is_setup(&mut self, initial_version: &str) -> Result<bool, ProtoError> {
        self.resolve_version(initial_version).await?;

        let install_dir = self.get_install_dir()?;

        if install_dir.exists() {
            self.find_bin_path().await?;

            let bin_path = {
                match self.get_bin_path() {
                    Ok(bin) => bin,
                    Err(_) => return Ok(false),
                }
            };

            if bin_path.exists() {
                self.create_shims().await?;

                return Ok(true);
            }
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
