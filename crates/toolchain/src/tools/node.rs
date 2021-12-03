use crate::errors::ToolchainError;
use crate::traits::Tool;
use async_trait::async_trait;
use monolith_config::workspace::NodeConfig;
use reqwest;
use std::env::consts;
use std::fs;
use std::io;
use std::path::PathBuf;

#[allow(unused_assignments)]
fn get_download_file_name(version: &String) -> Result<String, ToolchainError> {
	let mut platform = "";
	let mut ext = "tar.xz";

	if consts::OS == "linux" {
		platform = "linux"
	} else if consts::OS == "windows" {
		platform = "win";
		ext = "zip";
	} else if consts::OS == "macos" {
		platform = "darwin"
	} else {
		return Err(ToolchainError::UnsupportedPlatform(consts::OS.to_string()));
	}

	let mut arch = "";

	if consts::ARCH == "x86" {
		arch = "x86"
	} else if consts::ARCH == "x86_64" {
		arch = "x64"
	} else if consts::ARCH == "arm" {
		arch = "arm64"
	} else if consts::ARCH == "powerpc64" {
		arch = "ppc64le"
	} else if consts::ARCH == "s390x" {
		arch = "s390x"
	} else {
		return Err(ToolchainError::UnsupportedArchitecture(
			consts::ARCH.to_string(),
		));
	}

	Ok(format!(
		"node-v{version}-{platform}-{arch}.{ext}",
		version = version,
		platform = platform,
		arch = arch,
		ext = ext,
	))
}

#[derive(Debug)]
pub struct NodeTool {
	/// Path to the executable binary.
	bin_path: PathBuf,

	/// Path to the installation directory.
	install_dir: PathBuf,

	/// Version of the tool.
	version: String,
}

impl NodeTool {
	pub fn load(cache_dir: &PathBuf, config: &NodeConfig) -> Self {
		let mut install_dir = cache_dir.clone();

		install_dir.push("tools/node");
		install_dir.push(&config.version);

		let mut bin_path = install_dir.clone();

		if consts::OS == "windows" {
			bin_path.push("node.exe");
		} else {
			bin_path.push("bin/node");
		}

		NodeTool {
			bin_path,
			install_dir,
			version: String::from(&config.version),
		}
	}
}

#[async_trait]
impl Tool for NodeTool {
	fn is_downloaded(&self) -> bool {
		self.install_dir.exists()
	}

	async fn download(&self, temp_dir: &PathBuf) -> Result<PathBuf, ToolchainError> {
		let file_name = get_download_file_name(&self.version)?;
		let file_path = temp_dir.join(file_name);
		let mut file = fs::File::create(file_path)?;

		let response = reqwest::get(format!(
			"https://nodejs.org/dist/v{version}/{file_name}",
			version = self.version,
			file_name = file_name,
		))
		.await?;

		let mut content = io::Cursor::new(response.bytes().await?);

		io::copy(&mut content, &mut file)?;

		Ok(file_path)
	}

	fn is_installed(&self) -> bool {
		self.bin_path.exists()
	}

	async fn install(&self) -> Result<(), ToolchainError> {
		// TODO, unzip temp file
		Ok(())
	}

	fn get_bin_path(&self) -> &PathBuf {
		&self.bin_path
	}

	fn get_install_dir(&self) -> &PathBuf {
		&self.install_dir
	}
}
