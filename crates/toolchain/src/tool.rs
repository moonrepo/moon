use crate::errors::ToolchainError;
use crate::Toolchain;
use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait Tool {
    /// Returns an absolute file path to the executable binary for the tool.
    /// This _may not exist_, as the path is composed ahead of time.
    fn get_bin_path(&self) -> &PathBuf;

    /// Determine whether the tool has already been downloaded.
    fn is_downloaded(&self) -> bool;

    /// Downloads the tool into the ~/.moon/temp folder,
    /// and returns a file path to the downloaded binary.
    async fn download(&self, host: Option<&str>) -> Result<(), ToolchainError>;

    /// Returns an absolute file path to the temporary downloaded file.
    /// This _may not exist_, as the path is composed ahead of time.
    /// This is typically ~/.moon/temp/<file>.
    fn get_download_path(&self) -> Option<&PathBuf>;

    /// Determine whether the tool has already been installed.
    async fn is_installed(&self) -> Result<bool, ToolchainError>;

    /// Runs any installation steps after downloading.
    /// This is typically unzipping an archive, and running any installers/binaries.
    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError>;

    /// Returns an absolute file path to the directory containing the downloaded tool.
    /// This _may not exist_, as the path is composed ahead of time.
    /// This is typically ~/.moon/tools/<tool>/<version>.
    fn get_install_dir(&self) -> &PathBuf;

    /// Returns a semver version for the currently installed binary.
    /// This is typically acquired by executing the binary with a --version argument.
    async fn get_installed_version(&self) -> Result<String, ToolchainError>;
}

#[async_trait]
pub trait PackageManager {
    /// Dedupe dependencies after they have been installed.
    async fn dedupe_deps(&self, toolchain: &Toolchain) -> Result<(), ToolchainError>;

    /// Return the dependency range to use when linking local workspace packages.
    fn get_workspace_dependency_range(&self) -> String;

    /// Install dependencies at the root where a `package.json` exists.
    async fn install_deps(&self, toolchain: &Toolchain) -> Result<(), ToolchainError>;
}
