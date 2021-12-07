use crate::errors::ToolchainError;
use crate::tool::Tool;
use crate::Toolchain;
use async_trait::async_trait;
use flate2::read::GzDecoder;
use monolith_config::workspace::NodeConfig;
use reqwest;
use std::env::consts;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tar::Archive;

#[allow(unused_assignments)]
fn get_download_file_name(version: &str) -> Result<String, ToolchainError> {
    let mut platform = "";
    let mut ext = "tar.gz";

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

async fn download_file(
    download_path: &Path,
    version: &str,
    file_name: &str,
) -> Result<(), ToolchainError> {
    let mut file = fs::File::create(download_path).map_err(|_| ToolchainError::FailedToDownload)?;

    // Fetch the file from the HTTP distro
    let response = reqwest::get(format!(
        "https://nodejs.org/dist/v{version}/{file_name}",
        version = version,
        file_name = file_name,
    ))
    .await
    .map_err(|_| ToolchainError::FailedToDownload)?;

    // Write the bytes to our temp dir
    let mut contents = io::Cursor::new(
        response
            .bytes()
            .await
            .map_err(|_| ToolchainError::FailedToDownload)?,
    );

    io::copy(&mut contents, &mut file).map_err(|_| ToolchainError::FailedToDownload)?;

    Ok(())
}

// https://github.com/nodejs/node#verifying-binaries
fn verify_shasum(download_path: &Path, shasum_path: &Path) -> Result<(), ToolchainError> {
    let grep = Command::new("grep")
        .arg(download_path)
        .arg(shasum_path)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| ToolchainError::InvalidShasum)?;

    Command::new("sha256sum")
        .arg("-c")
        .arg("-")
        .stdin(grep.stdout.unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| ToolchainError::InvalidShasum)?;

    Ok(())
}

#[derive(Debug)]
pub struct NodeTool {
    bin_path: PathBuf,

    download_path: PathBuf,

    install_dir: PathBuf,

    pub version: String,
}

impl NodeTool {
    pub fn load(toolchain: &Toolchain, config: &NodeConfig) -> Result<NodeTool, ToolchainError> {
        let mut download_path = toolchain.temp_dir.clone();

        download_path.push("node");
        download_path.push(get_download_file_name(&config.version)?);

        let mut install_dir = toolchain.tools_dir.clone();

        install_dir.push("node");
        install_dir.push(&config.version);

        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("node.exe");
        } else {
            bin_path.push("bin/node");
        }

        Ok(NodeTool {
            bin_path,
            download_path,
            install_dir,
            version: String::from(&config.version),
        })
    }
}

#[async_trait]
impl Tool for NodeTool {
    fn is_downloaded(&self) -> bool {
        self.download_path.exists()
    }

    async fn download(&self) -> Result<(), ToolchainError> {
        // Download the node.tar.gz archive
        download_file(
            &self.download_path,
            &self.version,
            get_download_file_name(&self.version)?.as_str(),
        )
        .await?;

        // Download the SHASUMS256.txt file
        let shasum_path = self.download_path.parent().unwrap().join(format!(
            "node-v{version}-SHASUMS256.txt",
            version = self.version,
        ));

        download_file(&shasum_path, &self.version, "SHASUMS256.txt").await?;

        // Verify the binary
        verify_shasum(&self.download_path, &shasum_path)?;

        Ok(())
    }

    fn is_installed(&self) -> bool {
        self.install_dir.exists()
    }

    async fn install(&self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
        // Open .tar.gz file
        let tar_gz =
            fs::File::open(&self.download_path).map_err(|_| ToolchainError::FailedToInstall)?;

        // Decompress to .tar
        let tar = GzDecoder::new(tar_gz);

        // Unpack the archive into the install dir
        let mut archive = Archive::new(tar);

        archive
            .unpack(&self.install_dir)
            .map_err(|_| ToolchainError::FailedToInstall)?;

        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        &self.bin_path
    }

    fn get_download_path(&self) -> Option<&PathBuf> {
        Some(&self.download_path)
    }

    fn get_install_dir(&self) -> &PathBuf {
        &self.install_dir
    }
}
