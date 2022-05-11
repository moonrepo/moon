use crate::errors::ToolchainError;
use crate::helpers::get_path_env_var;
use crate::Toolchain;
use async_trait::async_trait;
use moon_logger::debug;
use moon_utils::process::Command;
use moon_utils::{fs, is_offline};
use std::path::PathBuf;

pub trait Logable {
    /// Return a unique name for logging.
    fn get_log_target(&self) -> String;
}

#[async_trait]
pub trait Downloadable<T: Send + Sync>: Send + Sync + Logable {
    /// Returns an absolute file path to the downloaded file.
    /// This _may not exist_, as the path is composed ahead of time.
    /// This is typically ~/.moon/temp/<file>.
    async fn get_download_path(&self) -> Result<PathBuf, ToolchainError>;

    /// Determine whether the tool has already been downloaded.
    async fn is_downloaded(&self) -> Result<bool, ToolchainError>;

    /// Downloads the tool into the ~/.moon/temp folder.
    async fn download(
        &self,
        parent: &T,
        host: Option<&str>, // Host to download from
    ) -> Result<(), ToolchainError>;

    /// Delete the downloaded file(s).
    async fn undownload(&self, parent: &T) -> Result<(), ToolchainError> {
        let download_path = self.get_download_path().await?;

        fs::remove_file(&download_path).await?;

        Ok(())
    }

    /// Run the download process: check if downloaded -> download or skip.
    async fn run_download(&self, parent: &T) -> Result<(), ToolchainError> {
        let target = self.get_log_target();

        if self.is_downloaded().await? {
            debug!(
                target: &target,
                "Tool has already been downloaded, continuing"
            );
        } else {
            debug!(target: &target, "Tool has not been downloaded, attempting");

            if is_offline() {
                return Err(ToolchainError::InternetConnectionRequired);
            }

            self.download(parent, None).await?;
        }

        Ok(())
    }

    /// Run the undownload process: check if downloaded -> delete files.
    async fn run_undownload(&self, parent: &T) -> Result<(), ToolchainError> {
        if self.is_downloaded().await? {
            self.undownload(parent).await?;

            debug!(target: &self.get_log_target(), "Deleted download files");
        }

        Ok(())
    }
}

#[async_trait]
pub trait Installable<T: Send + Sync>: Send + Sync + Logable {
    /// Returns an absolute file path to the directory containing the installed tool.
    /// This is typically ~/.moon/tools/<tool>/<version>.
    async fn get_install_dir(&self) -> Result<PathBuf, ToolchainError>;

    /// Returns a semver version for the currently installed binary.
    /// This is typically acquired by executing the binary with a `--version` argument.
    async fn get_installed_version(&self) -> Result<String, ToolchainError>;

    /// Determine whether the tool has already been installed.
    /// If `check_version` is false, avoid running the binaries as child processes
    /// to extract the current version.
    async fn is_installed(&self, check_version: bool) -> Result<bool, ToolchainError>;

    /// Runs any installation steps after downloading.
    /// This is typically unzipping an archive, and running any installers/binaries.
    async fn install(&self, parent: &T) -> Result<(), ToolchainError>;

    /// Delete the installation.
    async fn uninstall(&self, parent: &T) -> Result<(), ToolchainError> {
        let install_dir = self.get_install_dir().await?;

        fs::remove_dir_all(&install_dir).await?;

        Ok(())
    }

    /// Run the install process: check if installed & on the correct version ->
    /// install or skip. Return `true` if the tool was installed.
    async fn run_install(&self, parent: &T, check_version: bool) -> Result<bool, ToolchainError> {
        let target = self.get_log_target();

        if self.is_installed(check_version).await? {
            debug!(
                target: &target,
                "Tool has already been installed, continuing"
            );
        } else {
            debug!(target: &target, "Tool has not been installed, attempting");

            if is_offline() {
                return Err(ToolchainError::InternetConnectionRequired);
            }

            self.install(parent).await?;

            return Ok(true);
        }

        Ok(false)
    }

    /// Run the uninstall process: check if installed -> uninstall.
    async fn run_uninstall(&self, parent: &T) -> Result<(), ToolchainError> {
        if self.is_installed(false).await? {
            self.uninstall(parent).await?;

            debug!(target: &self.get_log_target(), "Uninstalled tool");
        }

        Ok(())
    }
}

#[async_trait]
pub trait Executable<T: Send + Sync>: Send + Sync {
    /// Find the absolute file path to the binary that will be executed.
    /// This happens after a tool has been downloaded/installed.
    async fn find_bin_path(&mut self, parent: &T) -> Result<(), ToolchainError>;

    /// Returns an absolute file path to the executable binary for the tool.
    fn get_bin_path(&self) -> &PathBuf;

    /// Return true if the binary exists and is executable.
    fn is_executable(&self) -> bool;
}

#[async_trait]
pub trait Lifecycle<T: Send + Sync>: Send + Sync {
    /// Setup the tool once it has been downloaded and installed.
    /// Return a count of how many sub-tools were installed.
    async fn setup(&mut self, _parent: &T, _check_version: bool) -> Result<u8, ToolchainError> {
        Ok(0)
    }

    /// Teardown the tool once it has been uninstalled.
    async fn teardown(&self, _parent: &T) -> Result<(), ToolchainError> {
        Ok(())
    }
}

struct ToolchainMetadata {}

#[async_trait]
pub trait Tool:
    Send
    + Sync
    + Logable
    + Downloadable<Toolchain>
    + Installable<Toolchain>
    + Executable<Toolchain>
    + Lifecycle<Toolchain>
{
    /// Download and install the tool within the toolchain.
    /// Once complete, trigger the setup hook, and return a count
    /// of how many sub-tools were installed.
    async fn run_setup(
        &mut self,
        toolchain: &Toolchain,
        check_version: bool,
    ) -> Result<u8, ToolchainError> {
        let mut installed = 0;

        self.run_download(toolchain).await?;

        if self.run_install(toolchain, check_version).await? {
            installed += 1;
        }

        self.find_bin_path(toolchain).await?;

        installed += self
            .setup(toolchain, installed > 0 || check_version)
            .await?;

        Ok(installed)
    }

    /// Teardown the tool by removing any downloaded/installed artifacts.
    /// This can be ran manually, or automatically during a failed load.
    async fn run_teardown(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        self.run_undownload(toolchain).await?;
        self.run_uninstall(toolchain).await?;
        self.teardown(toolchain).await?;

        Ok(())
    }
}

#[async_trait]
pub trait PackageManager<T: Send + Sync>:
    Send + Sync + Logable + Installable<T> + Executable<T> + Lifecycle<T>
{
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

    /// Install the package manager within the tool. Once complete,
    /// trigger the setup hook, and return a count
    /// of how many sub-tools were installed.
    async fn run_setup(&mut self, parent: &T, check_version: bool) -> Result<u8, ToolchainError> {
        let mut installed = 0;

        if self.run_install(parent, check_version).await? {
            installed += 1;
        }

        self.find_bin_path(parent).await?;

        installed += self.setup(parent, check_version).await?;

        Ok(installed)
    }

    /// Uninstall the package manager from the parent tool.
    async fn run_teardown(&self, parent: &T) -> Result<(), ToolchainError> {
        self.run_uninstall(parent).await?;
        self.teardown(parent).await?;

        Ok(())
    }
}
