use crate::errors::ToolchainError;
use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait Tool {
	/// Determine whether the tool has already been downloaded.
	fn is_downloaded(&self) -> bool;

	/// Downloads the tool into the ~/.monolith/temp folder,
	/// and return a file path to the downloaded binary.
	async fn download(&self, temp_dir: &PathBuf) -> Result<PathBuf, ToolchainError>;

	/// Determine whether the tool has already been installed.
	fn is_installed(&self) -> bool;

	/// Runs any installation steps after downloading.
	async fn install(&self) -> Result<(), ToolchainError>;

	/// Returns an absolute file path to the executable binary for the tool.
	fn get_bin_path(&self) -> &PathBuf;

	/// Returns an absolute file path to the directory containing the downloaded tool.
	/// This is typically ~/.monolith/tools/<tool>/<version>.
	fn get_install_dir(&self) -> &PathBuf;
}
