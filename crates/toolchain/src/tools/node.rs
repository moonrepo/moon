use crate::errors::ToolchainError;
use crate::helpers::{
    download_file_from_url, get_bin_version, get_file_sha256_hash, get_path_env_var, unpack,
};
use crate::tool::{Downloadable, Executable, Installable, Tool};
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
    bin_path: Option<PathBuf>,

    pub config: NodeConfig,
}

impl NodeTool {
    pub fn new(config: &NodeConfig) -> Result<NodeTool, ToolchainError> {
        Ok(NodeTool {
            bin_path: None,
            config: config.to_owned(),
        })
    }

    pub async fn exec_corepack<I, S>(&self, args: I) -> Result<(), ToolchainError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let bin_dir = self.get_bin_path().parent().unwrap().to_path_buf();

        let corepack_path = if cfg!(windows) {
            bin_dir.join("corepack.exe")
        } else {
            bin_dir.join("bin/corepack")
        };

        Command::new(&corepack_path)
            .args(args)
            .env("PATH", get_path_env_var(bin_dir))
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

    // pub fn get_npm(&self) -> &NpmTool {
    //     self.npm.as_ref().unwrap()
    // }

    // pub fn get_pnpm(&self) -> Option<&PnpmTool> {
    //     match &self.pnpm {
    //         Some(tool) => Some(tool),
    //         None => None,
    //     }
    // }

    // pub fn get_yarn(&self) -> Option<&YarnTool> {
    //     match &self.yarn {
    //         Some(tool) => Some(tool),
    //         None => None,
    //     }
    // }

    pub fn is_corepack_aware(&self) -> bool {
        let cfg_version = Version::parse(&self.config.version).unwrap();

        VersionReq::parse(">=16.9.0").unwrap().matches(&cfg_version)
            || VersionReq::parse("^14.19.0").unwrap().matches(&cfg_version)
    }
}

#[async_trait]
impl Downloadable for NodeTool {
    async fn get_download_path(&self, toolchain: &Toolchain) -> Result<PathBuf, ToolchainError> {
        Ok(toolchain
            .temp_dir
            .join("node")
            .join(get_download_file(&self.config.version)?))
    }

    async fn is_downloaded(&self, toolchain: &Toolchain) -> Result<bool, ToolchainError> {
        Ok(self.get_download_path(toolchain).await?.exists())
    }

    async fn download(
        &self,
        toolchain: &Toolchain,
        base_host: Option<&str>,
    ) -> Result<(), ToolchainError> {
        let version = &self.config.version;
        let host = base_host.unwrap_or("https://nodejs.org");
        let target = self.get_log_target();

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
            target: &target,
            "Verifying shasum against {}",
            color::url(&shasums_url),
        );

        // Verify the binary
        if let Err(error) = verify_shasum(&download_url, &download_path, &shasums_path) {
            error!(
                target: &target,
                "Shasum verification has failed. The downloaded file has been deleted, please try again."
            );

            fs::remove_file(&download_path).await?;

            return Err(error);
        }

        Ok(())
    }
}

#[async_trait]
impl Installable for NodeTool {
    async fn get_install_dir(&self, toolchain: &Toolchain) -> Result<PathBuf, ToolchainError> {
        Ok(toolchain.tools_dir.join("node").join(&self.config.version))
    }

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        Ok(get_bin_version(&self.get_bin_path()).await?)
    }

    async fn is_installed(
        &self,
        toolchain: &Toolchain,
        _check_version: bool,
    ) -> Result<bool, ToolchainError> {
        Ok(self.get_install_dir().await?.exists())
    }

    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let download_path = self.get_download_path(toolchain).await?;
        let install_dir = self.get_install_dir().await?;
        let prefix = get_download_file_name(&self.config.version)?;

        unpack(&download_path, &install_dir, &prefix).await?;

        debug!(
            target: &self.get_log_target(),
            "Unpacked and installed to {}",
            color::path(&install_dir)
        );

        Ok(())
    }
}

#[async_trait]
impl Executable for NodeTool {
    async fn find_bin_path(&mut self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let mut bin_path = self.get_install_dir().await?;

        if cfg!(windows) {
            bin_path.push("node.exe");
        } else {
            bin_path.push("bin/node");
        }

        self.bin_path = Some(bin_path);

        Ok(())
    }

    fn get_bin_path(&self) -> PathBuf {
        self.bin_path.unwrap()
    }
}

#[async_trait]
impl Tool for NodeTool {
    fn get_log_target(&self) -> String {
        String::from("moon:toolchain:node")
    }

    async fn setup(&self) -> Result<(), ToolchainError> {
        // let check_manager_version = installed_node || check_versions;

        // // Enable corepack before intalling package managers (when available)
        // if node.is_corepack_aware() && check_manager_version {
        //     debug!(
        //         target: "moon:toolchain:node",
        //         "Enabling corepack for package manager control"
        //     );

        //     node.exec_corepack(["enable"]).await?;
        // }

        // // Install npm (should always be available even if using another package manager)
        // let mut installed_pm = self
        //     .load_tool(self.get_npm(), check_manager_version)
        //     .await?;

        // // Install pnpm and yarn *after* setting the corepack package manager
        // if let Some(pnpm) = &self.pnpm {
        //     installed_pm = self.load_tool(pnpm, check_manager_version).await?;
        // }

        // if let Some(yarn) = &self.yarn {
        //     installed_pm = self.load_tool(yarn, check_manager_version).await?;
        // }

        Ok(())
    }
}
