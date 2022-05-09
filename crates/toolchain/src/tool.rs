use crate::errors::ToolchainError;
use crate::helpers::get_path_env_var;
use crate::Toolchain;
use async_trait::async_trait;
use moon_utils::process::Command;
use std::path::PathBuf;
use moon_utils::is_offline;

#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns an absolute file path to the directory containing the executable binaries.
    fn get_bin_dir(&self) -> PathBuf {
        self.get_bin_path().parent().unwrap().to_path_buf()
    }

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
    /// If `check_version` is false, avoid running the binaries as child processes
    /// to extract the current version.
    async fn is_installed(&self, check_version: bool) -> Result<bool, ToolchainError>;

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

    /// Load a tool into the toolchain by downloading an artifact/binary
    /// into the temp folder, then installing it into the tools folder.
    /// Return `true` if the tool was newly installed.
    async fn load(
        &mut self, 
        toolchain: &Toolchain,
        check_version: bool,
    ) -> Result<bool, ToolchainError> {
        if self.is_downloaded() {
            // Continue to install
        } else if is_offline() {
            return Err(ToolchainError::InternetConnectionRequired);
        } else {
            self.download(None).await?;
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

#[async_trait]
pub trait PackageManager: Tool {
    /// Create a command to run that wraps the tool binary.
    fn create_command(&self) -> Command {
        let mut cmd = Command::new(self.get_bin_path());
        cmd.env("PATH", get_path_env_var(self.get_bin_dir()));
        cmd
    }

    /// Dedupe dependencies after they have been installed.
    async fn dedupe_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError>;

    /// Download and execute a one-off package.
    async fn exec_package(
        &self,
        toolchain: &Toolchain,
        package: &str,
        args: Vec<&str>,
    ) -> Result<(), ToolchainError>;

    /// Return the name of the lockfile.
    fn get_lockfile_name(&self) -> String;

    /// Return the name of the manifest.
    fn get_manifest_name(&self) -> String {
        String::from("package.json")
    }

    /// Return the dependency range to use when linking local workspace packages.
    fn get_workspace_dependency_range(&self) -> String;

    /// Install dependencies for a defined manifest.
    async fn install_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError>;
}
