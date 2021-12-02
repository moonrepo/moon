use crate::errors::ToolchainError;
use std::path::PathBuf;

pub trait Tool {
	/// Determine whether the tool has already been downloaded.
	fn is_downloaded(&self) -> bool;

	/// Download the tool onto the local machine.
	fn download(&self) -> Result<(), ToolchainError>;

	/// Determin whether the tool has already been installed.
	fn is_installed(&self) -> bool;

	/// Run any installation steps after downloading.
	fn install(&self) -> Result<(), ToolchainError>;

	/// Returns an absolute file path to the executable binary for the tool.
	fn get_bin_path(&self) -> &PathBuf;

	/// Returns an absolute file path to the directory containing the downloaded tool.
	fn get_install_dir(&self) -> &PathBuf;
}
