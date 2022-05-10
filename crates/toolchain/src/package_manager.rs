use crate::errors::ToolchainError;
use crate::helpers::get_path_env_var;
use crate::Toolchain;
use async_trait::async_trait;
use moon_utils::is_offline;
use moon_utils::process::Command;
use std::path::PathBuf;

#[async_trait]
pub trait PackageManager {
    /// Create a command to run that wraps the binary.
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
