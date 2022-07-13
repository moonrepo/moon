use crate::errors::ToolchainError;
use crate::helpers::{
    download_file_from_url, get_bin_version, get_file_sha256_hash, get_path_env_var, unpack,
};
use crate::pms::npm::NpmTool;
use crate::pms::pnpm::PnpmTool;
use crate::pms::yarn::YarnTool;
use crate::traits::{Downloadable, Executable, Installable, Lifecycle, PackageManager, Tool};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::NodeConfig;
use moon_error::map_io_to_fs_error;
use moon_lang::LangError;
use moon_lang_node::node;
use moon_logger::{color, debug, error, Logable};
use moon_utils::fs;
use moon_utils::process::Command;
use semver::{Version, VersionReq};
use std::ffi::OsStr;
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
    pub fn new(toolchain: &Toolchain, config: &NodeConfig) -> Result<NodeTool, ToolchainError> {
        let install_dir = toolchain.tools_dir.join("node").join(&config.version);

        let mut node = NodeTool {
            bin_path: install_dir.join(node::get_bin_name_suffix("node", "exe", false)),
            config: config.to_owned(),
            download_path: toolchain
                .temp_dir
                .join("node")
                .join(node::get_download_file(&config.version)?),
            install_dir,
            log_target: String::from("moon:toolchain:node"),
            npm: None,
            pnpm: None,
            yarn: None,
        };

        node.npm = Some(NpmTool::new(&node, &config.npm)?);

        if let Some(pnpm_config) = &config.pnpm {
            node.pnpm = Some(PnpmTool::new(&node, pnpm_config)?);
        }

        if let Some(yarn_config) = &config.yarn {
            node.yarn = Some(YarnTool::new(&node, yarn_config)?);
        }

        Ok(node)
    }

    pub async fn exec_corepack<I, S>(&self, args: I) -> Result<(), ToolchainError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let corepack_path = node::find_package_manager_bin(&self.install_dir, "corepack");

        Command::new(&corepack_path)
            .args(args)
            .env("PATH", get_path_env_var(corepack_path.parent().unwrap()))
            .exec_capture_output()
            .await?;

        Ok(())
    }

    pub fn find_package_bin(
        &self,
        starting_dir: &Path,
        bin_name: &str,
    ) -> Result<PathBuf, ToolchainError> {
        match node::find_package_bin(starting_dir, bin_name) {
            Some(path) => Ok(path),
            None => Err(ToolchainError::MissingNodeModuleBin(bin_name.to_owned())),
        }
    }

    /// Return the `npm` package manager.
    pub fn get_npm(&self) -> &NpmTool {
        self.npm.as_ref().unwrap()
    }

    /// Return the `pnpm` package manager.
    pub fn get_pnpm(&self) -> Option<&PnpmTool> {
        match &self.pnpm {
            Some(tool) => Some(tool),
            None => None,
        }
    }

    /// Return the `yarn` package manager.
    pub fn get_yarn(&self) -> Option<&YarnTool> {
        match &self.yarn {
            Some(tool) => Some(tool),
            None => None,
        }
    }

    pub fn get_package_manager(&self) -> &(dyn PackageManager<Self> + Send + Sync) {
        if self.pnpm.is_some() {
            return self.get_pnpm().unwrap();
        }

        if self.yarn.is_some() {
            return self.get_yarn().unwrap();
        }

        self.get_npm()
    }

    #[track_caller]
    pub fn is_corepack_aware(&self) -> bool {
        let cfg_version = Version::parse(&self.config.version).unwrap();

        VersionReq::parse(">=16.9.0").unwrap().matches(&cfg_version)
            || VersionReq::parse("^14.19.0").unwrap().matches(&cfg_version)
    }
}

impl Logable for NodeTool {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }
}

#[async_trait]
impl Downloadable<Toolchain> for NodeTool {
    fn get_download_path(&self) -> Result<&PathBuf, ToolchainError> {
        Ok(&self.download_path)
    }

    async fn is_downloaded(&self) -> Result<bool, ToolchainError> {
        Ok(self.get_download_path()?.exists())
    }

    async fn download(
        &self,
        _toolchain: &Toolchain,
        base_host: Option<&str>,
    ) -> Result<(), ToolchainError> {
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
impl Installable<Toolchain> for NodeTool {
    fn get_install_dir(&self) -> Result<&PathBuf, ToolchainError> {
        Ok(&self.install_dir)
    }

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        Ok(get_bin_version(self.get_bin_path()).await?)
    }

    async fn is_installed(
        &self,
        _toolchain: &Toolchain,
        _check_version: bool,
    ) -> Result<bool, ToolchainError> {
        Ok(self.get_install_dir()?.exists())
    }

    async fn install(&self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
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
impl Executable<Toolchain> for NodeTool {
    async fn find_bin_path(&mut self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
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
impl Lifecycle<Toolchain> for NodeTool {
    async fn setup(
        &mut self,
        _toolchain: &Toolchain,
        check_version: bool,
    ) -> Result<u8, ToolchainError> {
        if self.is_corepack_aware() && check_version {
            debug!(
                target: self.get_log_target(),
                "Enabling corepack for package manager control"
            );

            self.exec_corepack(["enable"]).await?;
        }

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

impl Tool for NodeTool {}
