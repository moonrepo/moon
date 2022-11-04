use crate::errors::ToolchainError;
use crate::helpers::{download_file_from_url, get_file_sha256_hash, unpack};
use crate::pms::npm::NpmTool;
use crate::pms::pnpm::PnpmTool;
use crate::pms::yarn::YarnTool;
use crate::traits::{Downloadable, Executable, Installable, Lifecycle, PackageManager, Tool};
use crate::ToolchainPaths;
use async_trait::async_trait;
use moon_config::{NodeConfig, NodePackageManager};
use moon_error::map_io_to_fs_error;
use moon_lang::LangError;
use moon_logger::{color, debug, error, Logable};
use moon_node_lang::node;
use moon_utils::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

// https://github.com/nodejs/node#verifying-binaries
#[track_caller]
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

    Err(ToolchainError::Lang(LangError::InvalidShasum(
        String::from(download_path.to_string_lossy()),
        String::from(download_url),
    )))
}

#[derive(Debug)]
pub struct NodeTool {
    bin_path: PathBuf,

    pub config: NodeConfig,

    download_path: PathBuf,

    install_dir: PathBuf,

    log_target: String,

    npm: Option<NpmTool>,

    pnpm: Option<PnpmTool>,

    yarn: Option<YarnTool>,
}

impl NodeTool {
    pub fn new(paths: &ToolchainPaths, config: &NodeConfig) -> Result<NodeTool, ToolchainError> {
        let install_dir = paths.tools.join("node").join(&config.version);

        let mut node = NodeTool {
            bin_path: install_dir.join(node::get_bin_name_suffix("node", "exe", false)),
            config: config.to_owned(),
            download_path: paths
                .temp
                .join("node")
                .join(node::get_download_file(&config.version)?),
            install_dir,
            log_target: String::from("moon:toolchain:node"),
            npm: None,
            pnpm: None,
            yarn: None,
        };

        match config.package_manager {
            NodePackageManager::Npm => {
                node.npm = Some(NpmTool::new(paths, &config.npm)?);
            }
            NodePackageManager::Pnpm => {
                node.pnpm = Some(PnpmTool::new(paths, &config.pnpm)?);
            }
            NodePackageManager::Yarn => {
                node.yarn = Some(YarnTool::new(paths, &config.yarn)?);
            }
        };

        Ok(node)
    }

    pub fn find_package_bin(
        &self,
        starting_dir: &Path,
        bin_name: &str,
    ) -> Result<node::BinFile, ToolchainError> {
        match node::find_package_bin(starting_dir, bin_name)? {
            Some(bin) => Ok(bin),
            None => Err(ToolchainError::MissingNodeModuleBin(bin_name.to_owned())),
        }
    }

    /// Return the `npm` package manager.
    pub fn get_npm(&self) -> Option<&NpmTool> {
        self.npm.as_ref()
    }

    /// Return the `pnpm` package manager.
    pub fn get_pnpm(&self) -> Option<&PnpmTool> {
        self.pnpm.as_ref()
    }

    /// Return the `yarn` package manager.
    pub fn get_yarn(&self) -> Option<&YarnTool> {
        self.yarn.as_ref()
    }

    pub fn get_package_manager(&self) -> &(dyn PackageManager<Self> + Send + Sync) {
        if self.pnpm.is_some() {
            return self.get_pnpm().unwrap();
        }

        if self.yarn.is_some() {
            return self.get_yarn().unwrap();
        }

        if self.npm.is_some() {
            return self.get_npm().unwrap();
        }

        panic!("No package manager, how's this possible?");
    }
}

impl Logable for NodeTool {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }
}

#[async_trait]
impl Downloadable<()> for NodeTool {
    fn get_download_path(&self) -> Result<&PathBuf, ToolchainError> {
        Ok(&self.download_path)
    }

    async fn is_downloaded(&self) -> Result<bool, ToolchainError> {
        Ok(self.get_download_path()?.exists())
    }

    async fn download(&self, _parent: &(), base_host: Option<&str>) -> Result<(), ToolchainError> {
        let version = &self.config.version;
        let host = base_host.unwrap_or("https://nodejs.org");
        let log_target = self.get_log_target();

        // Download the node.tar.gz archive
        let download_url = node::get_nodejs_url(version, host, node::get_download_file(version)?);
        let download_path = self.get_download_path()?;

        download_file_from_url(&download_url, download_path).await?;

        // Download the SHASUMS256.txt file
        let shasums_url = node::get_nodejs_url(version, host, "SHASUMS256.txt");
        let shasums_path = download_path
            .parent()
            .unwrap()
            .join(format!("node-v{}-SHASUMS256.txt", version));

        download_file_from_url(&shasums_url, &shasums_path).await?;

        debug!(
            target: log_target,
            "Verifying shasum against {}",
            color::url(&shasums_url),
        );

        // Verify the binary
        if let Err(error) = verify_shasum(&download_url, download_path, &shasums_path) {
            error!(
                target: log_target,
                "Shasum verification has failed. The downloaded file has been deleted, please try again."
            );

            fs::remove_file(download_path).await?;

            return Err(error);
        }

        Ok(())
    }
}

#[async_trait]
impl Installable<()> for NodeTool {
    fn get_install_dir(&self) -> Result<&PathBuf, ToolchainError> {
        Ok(&self.install_dir)
    }

    async fn is_installed(
        &self,
        _parent: &(),
        _check_version: bool,
    ) -> Result<bool, ToolchainError> {
        Ok(self.get_install_dir()?.exists())
    }

    async fn install(&self, _parent: &()) -> Result<(), ToolchainError> {
        let download_path = self.get_download_path()?;
        let install_dir = self.get_install_dir()?;
        let prefix = node::get_download_file_name(&self.config.version)?;

        unpack(download_path, install_dir, &prefix).await?;

        debug!(
            target: self.get_log_target(),
            "Unpacked and installed to {}",
            color::path(install_dir)
        );

        Ok(())
    }
}

#[async_trait]
impl Executable<()> for NodeTool {
    async fn find_bin_path(&mut self, _parent: &()) -> Result<(), ToolchainError> {
        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        &self.bin_path
    }

    fn is_executable(&self) -> bool {
        true
    }
}

#[async_trait]
impl Lifecycle<()> for NodeTool {
    async fn setup(&mut self, _parent: &(), check_version: bool) -> Result<u8, ToolchainError> {
        let mut installed = 0;

        if self.npm.is_some() {
            let mut npm = self.npm.take().unwrap();
            installed += npm.run_setup(self, check_version).await?;
            self.npm = Some(npm);
        }

        if self.pnpm.is_some() {
            let mut pnpm = self.pnpm.take().unwrap();
            installed += pnpm.run_setup(self, check_version).await?;
            self.pnpm = Some(pnpm);
        }

        if self.yarn.is_some() {
            let mut yarn = self.yarn.take().unwrap();
            installed += yarn.run_setup(self, check_version).await?;
            self.yarn = Some(yarn);
        }

        Ok(installed)
    }
}

impl Tool for NodeTool {
    fn get_version(&self) -> String {
        self.config.version.clone()
    }
}
