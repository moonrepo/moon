use crate::errors::ToolchainError;
use crate::helpers::{download_file_from_url, get_bin_version, get_file_sha256_hash};
use crate::tool::Tool;
use crate::Toolchain;
use async_trait::async_trait;
use flate2::read::GzDecoder;
use monolith_config::workspace::NodeConfig;
use monolith_logger::{color, debug, error};
use std::env::consts;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tar::Archive;

fn get_download_file_ext() -> &'static str {
    if consts::OS == "windows" {
        "zip"
    } else {
        "tar.gz"
    }
}

#[allow(unused_assignments)]
fn get_download_file_name(version: &str) -> Result<String, ToolchainError> {
    let mut platform = "";

    if consts::OS == "linux" {
        platform = "linux"
    } else if consts::OS == "windows" {
        platform = "win";
    } else if consts::OS == "macos" {
        platform = "darwin"
    } else {
        return Err(ToolchainError::UnsupportedPlatform(
            consts::OS.to_string(),
            String::from("Node.js"),
        ));
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
            String::from("Node.js"),
        ));
    }

    Ok(format!(
        "node-v{version}-{platform}-{arch}",
        version = version,
        platform = platform,
        arch = arch,
    ))
}

fn get_download_file(version: &str) -> Result<String, ToolchainError> {
    Ok(format!(
        "{}.{}",
        get_download_file_name(version)?,
        get_download_file_ext()
    ))
}

fn get_nodejs_url(version: &str, host: &str, path: &str) -> String {
    format!(
        "{host}/dist/v{version}/{path}",
        host = host,
        version = version,
        path = path,
    )
}

// https://github.com/nodejs/node#verifying-binaries
fn verify_shasum(
    download_url: &str,
    download_path: &Path,
    shasums_path: &Path,
) -> Result<(), ToolchainError> {
    let file_name = download_path.file_name().unwrap().to_str().unwrap();
    let sha_hash = get_file_sha256_hash(download_path)?;

    for line in BufReader::new(fs::File::open(shasums_path)?)
        .lines()
        .flatten()
    {
        // hash1923hnsdouahsd91houn79h1beyasdpaksdm  node-vx.x.x-darwin-arm64.tar.gz
        if line.starts_with(sha_hash.as_str()) && line.ends_with(file_name) {
            return Ok(());
        }
    }

    Err(ToolchainError::InvalidShasum(
        String::from(download_path.to_string_lossy()),
        String::from(download_url),
    ))
}

#[derive(Clone, Debug)]
pub struct NodeTool {
    bin_path: PathBuf,

    download_path: PathBuf,

    install_dir: PathBuf,

    pub config: NodeConfig,
}

impl NodeTool {
    pub fn new(toolchain: &Toolchain, config: &NodeConfig) -> Result<NodeTool, ToolchainError> {
        let mut download_path = toolchain.temp_dir.clone();

        download_path.push("node");
        download_path.push(get_download_file(&config.version)?);

        let mut install_dir = toolchain.tools_dir.clone();

        install_dir.push("node");
        install_dir.push(&config.version);

        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("node.exe");
        } else {
            bin_path.push("bin/node");
        }

        debug!(
            target: "toolchain:node",
            "Creating tool at {}",
            color::file_path(&bin_path)
        );

        Ok(NodeTool {
            bin_path,
            config: config.to_owned(),
            download_path,
            install_dir,
        })
    }
}

#[async_trait]
impl Tool for NodeTool {
    fn is_downloaded(&self) -> bool {
        let exists = self.download_path.exists();

        if exists {
            debug!(
                target: "toolchain:node",
                "Binary has already been downloaded, continuing"
            );
        } else {
            debug!(
                target: "toolchain:node",
                "Binary does not exist, attempting to download"
            );
        }

        exists
    }

    async fn download(&self, base_host: Option<&str>) -> Result<(), ToolchainError> {
        let version = &self.config.version;
        let host = base_host.unwrap_or("https://nodejs.org");

        // Download the node.tar.gz archive
        let download_url = get_nodejs_url(version, host, &get_download_file(version)?);

        download_file_from_url(&download_url, &self.download_path).await?;

        debug!(
            target: "toolchain:node",
            "Downloading binary from {} to {}",
            color::url(&download_url),
            color::file_path(&self.download_path)
        );

        // Download the SHASUMS256.txt file
        let shasums_url = get_nodejs_url(version, host, "SHASUMS256.txt");
        let shasums_path = self
            .download_path
            .parent()
            .unwrap()
            .join(format!("node-v{}-SHASUMS256.txt", version));

        download_file_from_url(&shasums_url, &shasums_path).await?;

        debug!(
            target: "toolchain:node",
            "Verifying shasum against {}",
            color::url(&shasums_url),
        );

        // Verify the binary
        if let Err(error) = verify_shasum(&download_url, &self.download_path, &shasums_path) {
            error!(
                target: "toolchain:node",
                "Shasum verification has failed. The downloaded file has been deleted, please try again."
            );

            fs::remove_file(&self.download_path)?;

            return Err(error);
        }

        Ok(())
    }

    async fn is_installed(&self) -> Result<bool, ToolchainError> {
        let installed = self.install_dir.exists();
        let correct_version = self.get_installed_version().await? == self.config.version;

        if installed && correct_version {
            debug!(
                target: "toolchain:node",
                "Download has already been installed and is on the correct version",
            );

            return Ok(true);
        }

        if !installed {
            debug!(
                target: "toolchain:node",
                "Download has not been installed, attempting to install",
            );
        } else if !correct_version {
            debug!(
                target: "toolchain:node",
                "Download has been installed, but is on the wrong version, attempting to reinstall",
            );
        }

        Ok(false)
    }

    async fn install(&self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
        debug!(
            target: "toolchain:node",
            "Unpacking download and installing to {}",
            color::file_path(&self.install_dir)
        );

        // Open .tar.gz file
        let tar_gz = fs::File::open(&self.download_path)?;

        // Decompress to .tar
        let tar = GzDecoder::new(tar_gz);

        // Unpack the archive into the install dir
        let mut archive = Archive::new(tar);
        let prefix = get_download_file_name(&self.config.version)?;

        archive.entries().unwrap().for_each(|entry_result| {
            let mut entry = entry_result.unwrap();

            // Remove the download folder prefix from all files
            let path = entry
                .path()
                .unwrap()
                .strip_prefix(&prefix)
                .unwrap()
                .to_owned();

            entry.unpack(&self.install_dir.join(path)).unwrap();
        });

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

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        Ok(get_bin_version(self.get_bin_path()).await?)
    }
}
