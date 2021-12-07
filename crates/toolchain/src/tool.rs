use crate::errors::ToolchainError;
use crate::Toolchain;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait Tool {
    /// Returns an absolute file path to the executable binary for the tool.
    /// This _may not exist_, as the path is composed ahead of time.
    fn get_bin_path(&self) -> &PathBuf;

    /// Determine whether the tool has already been downloaded.
    fn is_downloaded(&self) -> bool;

    /// Downloads the tool into the ~/.monolith/temp folder,
    /// and returns a file path to the downloaded binary.
    async fn download(&self) -> Result<(), ToolchainError>;

    /// Returns an absolute file path to the temporary downloaded file.
    /// This _may not exist_, as the path is composed ahead of time.
    /// This is typically ~/.monolith/temp/<file>.
    fn get_download_path(&self) -> Option<&PathBuf>;

    /// Determine whether the tool has already been installed.
    fn is_installed(&self) -> bool;

    /// Runs any installation steps after downloading.
    /// This is typically unzipping an archive, and running any installers/binaries.
    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError>;

    /// Returns an absolute file path to the directory containing the downloaded tool.
    /// This _may not exist_, as the path is composed ahead of time.
    /// This is typically ~/.monolith/tools/<tool>/<version>.
    fn get_install_dir(&self) -> &PathBuf;
}

#[async_trait]
pub trait PackageManager {
    /// Install dependencies at the root where a `package.json` exists.
    async fn install_deps(&self, root_dir: &Path) -> Result<(), ToolchainError>;
}
