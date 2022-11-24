use crate::errors::ToolchainError;
use async_trait::async_trait;
use moon_lang::LockfileDependencyVersions;
use moon_utils::process::Command;
use std::path::Path;

#[async_trait]
pub trait RuntimeTool: Send + Sync {
    /// Return the resolved version of the current tool.
    fn get_version(&self) -> &str;

    /// Setup the tool by downloading and installing it.
    /// Return a count of how many sub-tools were installed.
    async fn setup(&mut self) -> Result<u8, ToolchainError> {
        Ok(0)
    }

    /// Teardown the tool by uninstalling and deleting files.
    async fn teardown(&mut self) -> Result<(), ToolchainError> {
        Ok(())
    }
}

#[async_trait]
pub trait DependencyManager<T: Send + Sync>: Send + Sync + RuntimeTool {
    /// Create a command to run that wraps the binary.
    fn create_command(&self, parent: &T) -> Command;

    /// Dedupe dependencies after they have been installed.
    async fn dedupe_dependencies(
        &self,
        parent: &T,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolchainError>;

    /// Return the name of the lockfile.
    fn get_lock_filename(&self) -> String;

    /// Return the name of the manifest.
    fn get_manifest_filename(&self) -> String;

    /// Return a list of dependencies resolved to their latest version from the lockfile.
    /// Dependencies are based on a manifest at the provided path.
    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> Result<LockfileDependencyVersions, ToolchainError>;

    /// Install dependencies for a defined manifest.
    async fn install_dependencies(
        &self,
        parent: &T,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolchainError>;

    /// Install dependencies for a single package in the workspace.
    async fn install_focused_dependencies(
        &self,
        parent: &T,
        packages: &[String],
        production_only: bool,
    ) -> Result<(), ToolchainError>;
}
