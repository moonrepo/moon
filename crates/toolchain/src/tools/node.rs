use crate::errors::ToolchainError;
use crate::helpers::{
    download_file_from_url, get_bin_version, get_file_sha256_hash, get_path_env_var, unpack,
};
use crate::tool::{Downloadable, Installable, Tool};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::constants::CONFIG_DIRNAME;
use moon_config::NodeConfig;
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, error};
use moon_utils::fs;
use moon_utils::process::Command;
use semver::{Version, VersionReq};
use std::env::consts;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

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
    let file_handle =
        File::open(shasums_path).map_err(|e| map_io_to_fs_error(e, shasums_path.to_path_buf()))?;

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

    install_dir: PathBuf,

    pub config: NodeConfig,
}

impl NodeTool {
    pub fn new(toolchain: &Toolchain, config: &NodeConfig) -> Result<NodeTool, ToolchainError> {
        let mut install_dir = toolchain.tools_dir.clone();

        install_dir.push("node");
        install_dir.push(&config.version);

        let mut bin_path = install_dir.clone();
        let mut corepack_bin_path = install_dir.clone();

        if cfg!(windows) {
            bin_path.push("node.exe");
            corepack_bin_path.push("corepack.cmd");
        } else {
            bin_path.push("bin/node");
            corepack_bin_path.push("bin/corepack");
        }

        debug!(
            target: "moon:toolchain:node",
            "Creating tool at {}",
            color::path(&bin_path)
        );

        Ok(NodeTool {
            bin_path,
            corepack_bin_path,
            config: config.to_owned(),
            install_dir,
        })
    }

    pub async fn exec_corepack<I, S>(&self, args: I) -> Result<(), ToolchainError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new(&self.corepack_bin_path)
            .args(args)
            .env("PATH", get_path_env_var(self.get_bin_dir()))
            .exec_capture_output()
            .await?;

        Ok(())
    }

    pub fn find_package_bin_path(
        &self,
        package_name: &str,
        starting_dir: &Path,
    ) -> Result<PathBuf, ToolchainError> {
        let mut bin_path = starting_dir.join("node_modules").join(".bin");

        if cfg!(windows) {
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
        let cfg_version = Version::parse(&self.config.version).unwrap();

        VersionReq::parse(">=16.9.0").unwrap().matches(&cfg_version)
            || VersionReq::parse("^14.19.0").unwrap().matches(&cfg_version)
    }
}

#[async_trait]
impl Downloadable for NodeTool {
    async fn is_downloaded(&self, toolchain: &Toolchain) -> Result<bool, ToolchainError> {
        Ok(self.get_download_path(toolchain).await?.exists())
    }

    async fn download(
        &self,
        toolchain: &Toolchain,
        base_host: Option<&str>,
    ) -> Result<PathBuf, ToolchainError> {
        let version = &self.config.version;
        let host = base_host.unwrap_or("https://nodejs.org");

        // Download the node.tar.gz archive
        let download_url = get_nodejs_url(version, host, &get_download_file(version)?);
        let download_path = self.get_download_path(toolchain).await?;

        download_file_from_url(&download_url, &download_path).await?;

        // Download the SHASUMS256.txt file
        let shasums_url = get_nodejs_url(version, host, "SHASUMS256.txt");
        let shasums_path = download_path
            .parent()
            .unwrap()
            .join(format!("node-v{}-SHASUMS256.txt", version));

        download_file_from_url(&shasums_url, &shasums_path).await?;

        debug!(
            target: self.get_log_target(),
            "Verifying shasum against {}",
            color::url(&shasums_url),
        );

        // Verify the binary
        if let Err(error) = verify_shasum(&download_url, &download_path, &shasums_path) {
            error!(
                target: self.get_log_target(),
                "Shasum verification has failed. The downloaded file has been deleted, please try again."
            );

            fs::remove_file(&download_path).await?;

            return Err(error);
        }

        Ok(download_path)
    }

    async fn get_download_path(&self, toolchain: &Toolchain) -> Result<PathBuf, ToolchainError> {
        toolchain
            .temp_dir
            .join("node")
            .join(get_download_file(&self.config.version)?);
    }
}

#[async_trait]
impl Tool for NodeTool {
    async fn is_installed(&self, _check_version: bool) -> Result<bool, ToolchainError> {
        if self.install_dir.exists() {
            debug!(
                target: "moon:toolchain:node",
                "Download has already been installed and is on the correct version",
            );

            return Ok(true);
        }

        debug!(
            target: "moon:toolchain:node",
            "Download has not been installed",
        );

        Ok(false)
    }

    async fn install(&self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let install_dir = self.get_install_dir();
        let prefix = get_download_file_name(&self.config.version)?;

        unpack(&download_path, install_dir, &prefix).await?;

        debug!(
            target: "moon:toolchain:node",
            "Unpacked and installed to {}",
            color::path(install_dir)
        );

        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        &self.bin_path
    }

    fn get_log_target(&self) -> String {
        String::from("moon:toolchain:node")
    }

    fn get_install_dir(&self) -> &PathBuf {
        &self.install_dir
    }

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        Ok(get_bin_version(self.get_bin_path()).await?)
    }
}
