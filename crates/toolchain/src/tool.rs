use crate::errors::ToolchainError;
use crate::helpers::get_path_env_var;
use crate::Toolchain;
use async_trait::async_trait;
use moon_logger::{color, debug, error};
use moon_utils::is_offline;
use moon_utils::process::Command;
use std::path::PathBuf;

#[async_trait]
pub trait Downloadable: Send + Sync {
    /// Determine whether the tool has already been downloaded.
    async fn is_downloaded(&self, toolchain: &Toolchain) -> Result<bool, ToolchainError>;

    /// Downloads the tool into the ~/.moon/temp folder,
    /// and returns a file path to the downloaded binary.
    async fn download(
        &self,
        toolchain: &Toolchain,
        host: Option<&str>,
    ) -> Result<PathBuf, ToolchainError>;

    /// Returns an absolute file path to the temporary downloaded file.
    /// This _may not exist_, as the path is composed ahead of time.
    /// This is typically ~/.moon/temp/<file>.
    async fn get_download_path(&self, toolchain: &Toolchain) -> Result<PathBuf, ToolchainError>;
}

#[async_trait]
pub trait Installable: Send + Sync {
    /// Determine whether the tool has already been installed.
    /// If `check_version` is false, avoid running the binaries as child processes
    /// to extract the current version.
    async fn is_installed(&self, check_version: bool) -> Result<bool, ToolchainError>;

    /// Runs any installation steps after downloading.
    /// This is typically unzipping an archive, and running any installers/binaries.
    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError>;

    /// Returns an absolute file path to the directory containing the downloaded tool.
    /// This is typically ~/.moon/tools/<tool>/<version>.
    fn get_install_dir(&self) -> &PathBuf;

    /// Returns a semver version for the currently installed binary.
    /// This is typically acquired by executing the binary with a `--version` argument.
    async fn get_installed_version(&self) -> Result<String, ToolchainError>;
}

#[async_trait]
pub trait Tool: Send + Sync + Downloadable + Installable {
    /// Returns an absolute file path to the executable binary for the tool.
    /// This _may not exist_, as the path is composed ahead of time.
    fn get_bin_path(&self) -> &PathBuf;

    /// Returns an absolute file path to the directory containing the executable binaries.
    fn get_bin_dir(&self) -> PathBuf {
        self.get_bin_path().parent().unwrap().to_path_buf()
    }

    fn get_log_target(&self) -> String;

    /// Load a tool into the toolchain by downloading an artifact/binary
    /// into the temp folder, then installing it into the tools folder.
    /// Return `true` if the tool was newly installed.
    async fn load(
        &mut self,
        toolchain: &Toolchain,
        check_version: bool,
    ) -> Result<bool, ToolchainError> {
        let target = self.get_log_target();

        if self.is_downloaded(toolchain).await? {
            debug!(
                target: target,
                "Tool has already been downloaded, continuing"
            );
        } else {
            debug!(
                target: target,
                "Tool does not exist, attempting to download"
            );

            if is_offline() {
                return Err(ToolchainError::InternetConnectionRequired);
            }

            self.download(toolchain, None).await?;
        }

        if self.is_installed(check_version).await? {
            return Ok(false);
        } else if is_offline() {
            return Err(ToolchainError::InternetConnectionRequired);
        } else {
            self.install(toolchain).await?;
        }

        Ok(true)
    }
}
