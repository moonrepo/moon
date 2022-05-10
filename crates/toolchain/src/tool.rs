use crate::errors::ToolchainError;
use crate::helpers::get_path_env_var;
use crate::Toolchain;
use async_trait::async_trait;
use moon_logger::debug;
use moon_utils::process::Command;
use moon_utils::{fs, is_offline};
use std::path::PathBuf;

#[async_trait]
pub trait Downloadable: Send + Sync {
    /// Returns an absolute file path to the downloaded file.
    /// This _may not exist_, as the path is composed ahead of time.
    /// This is typically ~/.moon/temp/<file>.
    async fn get_download_path(&self, toolchain: &Toolchain) -> Result<PathBuf, ToolchainError>;

    /// Determine whether the tool has already been downloaded.
    async fn is_downloaded(&self, toolchain: &Toolchain) -> Result<bool, ToolchainError>;

    /// Downloads the tool into the ~/.moon/temp folder.
    async fn download(
        &self,
        toolchain: &Toolchain,
        host: Option<&str>, // Host to download from
    ) -> Result<(), ToolchainError>;

    /// Delete the downloaded file(s).
    async fn undownload(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let download_path = self.get_download_path(toolchain).await?;

        fs::remove_file(&download_path).await?;

        Ok(())
    }
}

#[async_trait]
pub trait Installable: Send + Sync {
    /// Returns an absolute file path to the directory containing the instaled tool.
    /// This is typically ~/.moon/tools/<tool>/<version>.
    async fn get_install_dir(&self) -> Result<PathBuf, ToolchainError>;

    /// Returns a semver version for the currently installed binary.
    /// This is typically acquired by executing the binary with a `--version` argument.
    async fn get_installed_version(&self) -> Result<String, ToolchainError>;

    /// Determine whether the tool has already been installed.
    /// If `check_version` is false, avoid running the binaries as child processes
    /// to extract the current version.
    async fn is_installed(
        &self,
        toolchain: &Toolchain,
        check_version: bool,
    ) -> Result<bool, ToolchainError>;

    /// Runs any installation steps after downloading.
    /// This is typically unzipping an archive, and running any installers/binaries.
    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError>;

    /// Delete the installation.
    async fn uninstall(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let install_dir = self.get_install_dir().await?;

        fs::remove_dir_all(&install_dir).await?;

        Ok(())
    }
}

#[async_trait]
pub trait Executable: Send + Sync {
    /// Find the absolute file path to the binary that will be executed.
    /// This happens after a tool has been downloaded/installed.
    async fn find_bin_path(&mut self, toolchain: &Toolchain) -> Result<(), ToolchainError>;

    /// Returns an absolute file path to the executable binary for the tool.
    fn get_bin_path(&self) -> PathBuf;
}

#[async_trait]
pub trait Tool: Send + Sync + Downloadable + Installable + Executable {
    /// Return a unique name for logging.
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
        let mut installed = false;

        if self.is_downloaded(toolchain).await? {
            debug!(
                target: &target,
                "Tool has already been downloaded, continuing"
            );
        } else {
            debug!(target: &target, "Tool has not been downloaded, attempting");

            if is_offline() {
                return Err(ToolchainError::InternetConnectionRequired);
            }

            self.download(toolchain, None).await?;
        }

        if self.is_installed(toolchain, check_version).await? {
            debug!(
                target: &target,
                "Tool has already been installed, continuing"
            );
        } else {
            debug!(target: &target, "Tool has not been installed, attempting");

            if is_offline() {
                return Err(ToolchainError::InternetConnectionRequired);
            }

            self.install(toolchain).await?;
            installed = true;
        }

        self.find_bin_path(toolchain).await?;
        self.setup().await?;

        Ok(installed)
    }

    /// Setup the tool once it has been loaded.
    async fn setup(&self) -> Result<(), ToolchainError> {
        Ok(())
    }

    /// Teardown the tool once it has been unloaded.
    async fn teardown(&self) -> Result<(), ToolchainError> {
        Ok(())
    }

    /// Unload the tool by removing any downloaded/installed artifacts.
    /// This can be ran manually, or automatically during a failed load.
    async fn unload(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let target = self.get_log_target();

        if self.is_downloaded(toolchain).await? {
            self.undownload(toolchain).await?;

            debug!(target: &target, "Deleted download files");
        }

        if self.is_installed(toolchain, false).await? {
            self.uninstall(toolchain).await?;

            debug!(target: &target, "Deleted installation");
        }

        self.teardown().await?;

        Ok(())
    }
}

#[async_trait]
pub trait PackageManager: Send + Sync + Installable + Executable {
    /// Create a command to run that wraps the binary.
    fn create_command(&self) -> Command {
        let bin_path = self.get_bin_path();

        let mut cmd = Command::new(bin_path);
        cmd.env(
            "PATH",
            get_path_env_var(bin_path.parent().unwrap().to_path_buf()),
        );
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
