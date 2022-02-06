use crate::errors::ToolchainError;
use crate::helpers::{download_file_from_url, get_bin_version, get_file_sha256_hash};
use crate::tool::Tool;
use crate::Toolchain;
use async_trait::async_trait;
use flate2::read::GzDecoder;
use moon_config::constants::CONFIG_DIRNAME;
use moon_config::NodeConfig;
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, error};
use moon_utils::fs;
use moon_utils::process::{create_command, exec_command, Output};
use semver::{Version, VersionReq};
use std::env::consts;
use std::ffi::OsStr;
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
    let sha_hash = get_file_sha256_hash(download_path)?;
    let file_name = download_path.file_name().unwrap().to_str().unwrap();
    let file_handle = fs::File::open(shasums_path)
        .map_err(|e| map_io_to_fs_error(e, shasums_path.to_path_buf()))?;

    for line in BufReader::new(file_handle).lines().flatten() {
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

    corepack_bin_path: PathBuf,

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
        let mut corepack_bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("node.exe");
            corepack_bin_path.push("corepack.exe");
        } else {
            bin_path.push("bin/node");
            corepack_bin_path.push("bin/corepack");
        }

        debug!(
            target: "moon:toolchain:node",
            "Creating tool at {}",
            color::file_path(&bin_path)
        );

        Ok(NodeTool {
            bin_path,
            corepack_bin_path,
            config: config.to_owned(),
            download_path,
            install_dir,
        })
    }

    pub async fn exec_corepack<I, S>(&self, args: I) -> Result<Output, ToolchainError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Ok(exec_command(create_command(&self.corepack_bin_path).args(args)).await?)
    }

    pub fn find_package_bin_path(
        &self,
        package_name: &str,
        starting_dir: &Path,
    ) -> Result<PathBuf, ToolchainError> {
        let mut bin_path = starting_dir.join("node_modules/.bin");

        if consts::OS == "windows" {
            bin_path.push(format!("{}.cmd", package_name));
        } else {
            bin_path.push(package_name);
        }

        if bin_path.exists() {
            return Ok(bin_path);
        }

        // If we've reached the root of the workspace, and still haven't found
        // a binary, just abort with an error...
        if starting_dir.join(CONFIG_DIRNAME).exists() {
            return Err(ToolchainError::MissingNodeModuleBin(String::from(
                package_name,
            )));
        }

        self.find_package_bin_path(package_name, starting_dir.parent().unwrap())
    }

    pub fn is_corepack_aware(&self) -> bool {
        let min_version = VersionReq::parse(">=16.9.0").unwrap();
        let cfg_version = Version::parse(&self.config.version).unwrap();

        min_version.matches(&cfg_version)
    }
}

#[async_trait]
impl Tool for NodeTool {
    fn is_downloaded(&self) -> bool {
        let exists = self.download_path.exists();

        if exists {
            debug!(
                target: "moon:toolchain:node",
                "Binary has already been downloaded, continuing"
            );
        } else {
            debug!(
                target: "moon:toolchain:node",
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
            target: "moon:toolchain:node",
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
            target: "moon:toolchain:node",
            "Verifying shasum against {}",
            color::url(&shasums_url),
        );

        // Verify the binary
        if let Err(error) = verify_shasum(&download_url, &self.download_path, &shasums_path) {
            error!(
                target: "moon:toolchain:node",
                "Shasum verification has failed. The downloaded file has been deleted, please try again."
            );

            fs::remove_file(&self.download_path).await?;

            return Err(error);
        }

        Ok(())
    }

    async fn is_installed(&self) -> Result<bool, ToolchainError> {
        if self.install_dir.exists() {
            let version = self.get_installed_version().await?;

            if version == self.config.version {
                debug!(
                    target: "moon:toolchain:node",
                    "Download has already been installed and is on the correct version",
                );

                return Ok(true);
            }

            debug!(
                target: "moon:toolchain:node",
                "Download has been installed, but is on the wrong version ({}), attempting to reinstall",
                version,
            );
        } else {
            debug!(
                target: "moon:toolchain:node",
                "Download has not been installed",
            );
        }

        Ok(false)
    }

    async fn install(&self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let install_dir = self.get_install_dir();

        fs::create_dir_all(install_dir).await?;

        // Open .tar.gz file
        let tar_gz = fs::File::open(&self.download_path)
            .map_err(|e| map_io_to_fs_error(e, self.download_path.clone()))?;

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

            entry.unpack(&self.get_install_dir().join(path)).unwrap();
        });

        debug!(
            target: "moon:toolchain:node",
            "Unpacked and installed to {}",
            color::file_path(self.get_install_dir())
        );

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
