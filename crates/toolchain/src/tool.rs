use crate::errors::ToolchainError;
use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Tool {
	/// Path to the executable binary.
	/// This _may not exist_, as the path is composed ahead of time.
	pub bin_path: PathBuf,

	/// Path to the downloaded file name.
	/// This _may not exist_, as the path is composed ahead of time.
	pub download_path: Option<PathBuf>,

	/// Path to the installation directory.
	/// This _may not exist_, as the path is composed ahead of time.
	pub install_dir: PathBuf,

	/// Version of the tool.
	pub version: String,
}

#[async_trait]
pub trait ToolRuntime {
	/// Returns an absolute file path to the executable binary for the tool.
	fn get_bin_path(&self) -> &PathBuf;

	/// Determine whether the tool has already been downloaded.
	fn is_downloaded(&self) -> bool;

	/// Downloads the tool into the ~/.monolith/temp folder,
	/// and returns a file path to the downloaded binary.
	async fn download(&self) -> Result<(), ToolchainError>;

	/// Returns an absolute file path to the temporary downloaded file.
	/// This is typically ~/.monolith/temp/<file>.
	fn get_download_path(&self) -> Option<&PathBuf>;

	/// Determine whether the tool has already been installed.
	fn is_installed(&self) -> bool;

	/// Runs any installation steps after downloading.
	/// This is typically unzipping an archive, and running any installers/binaries.
	async fn install(&self) -> Result<(), ToolchainError>;

	/// Returns an absolute file path to the directory containing the downloaded tool.
	/// This is typically ~/.monolith/tools/<tool>/<version>.
	fn get_install_dir(&self) -> &PathBuf;

	async fn load(&self) -> Result<(), ToolchainError> {
		if !self.is_downloaded() {
			self.download().await?;
		}

		if !self.is_installed() {
			self.install().await?;
		}

		Ok(())
	}

	/// Unload the tool by removing any downloaded/installed artifacts.
	/// This can be ran manually, or automatically during a failed load.
	async fn unload(&self) -> Result<(), ToolchainError> {
		let download_path = self.get_download_path();

		if self.is_downloaded() && download_path.is_some() {
			fs::remove_file(download_path.unwrap()).map_err(|_| ToolchainError::FailedToUnload)?;
		}

		if self.is_installed() {
			fs::remove_dir_all(self.get_install_dir()).map_err(|_| ToolchainError::FailedToUnload)?;
		}

		Ok(())
	}
}
