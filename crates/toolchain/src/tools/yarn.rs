use crate::errors::ToolchainError;
use crate::helpers::{exec_command, get_bin_version, is_ci};
use crate::tool::{PackageManager, Tool};
use crate::Toolchain;
use async_trait::async_trait;
use monolith_config::workspace::YarnConfig;
use std::env::consts;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct YarnTool {
    bin_path: PathBuf,

    install_dir: PathBuf,

    pub config: YarnConfig,
}

impl YarnTool {
    pub fn new(toolchain: &Toolchain, config: &YarnConfig) -> Result<YarnTool, ToolchainError> {
        let install_dir = toolchain.get_node().get_install_dir().clone();
        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("yarn");
        } else {
            bin_path.push("bin/yarn");
        }

        Ok(YarnTool {
            bin_path,
            config: config.to_owned(),
            install_dir,
        })
    }

    fn is_v1(&self) -> bool {
        self.config.version.starts_with('1')
    }
}

#[async_trait]
impl Tool for YarnTool {
    fn is_downloaded(&self) -> bool {
        true
    }

    async fn download(&self, _host: Option<&str>) -> Result<(), ToolchainError> {
        Ok(()) // This is handled by node
    }

    async fn is_installed(&self) -> Result<bool, ToolchainError> {
        Ok(self.bin_path.exists() && self.get_installed_version().await? == self.config.version)
    }

    // Yarn is installed through npm, but only v1 exists in the npm registry,
    // even if a consumer is using Yarn 2/3. https://www.npmjs.com/package/yarn
    // Yarn >= 2 work differently than normal packages, as their runtime code
    // is stored *within* the repository, and the v1 package detects it.
    // Because of this, we need to always install the v1 package!
    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let npm = toolchain.get_npm();

        if self.is_v1() {
            // npm install -g yarn@1.x.x
            npm.add_global_dep("yarn", &self.config.version).await?;
        } else {
            // npm install -g yarn@latest
            npm.add_global_dep("yarn", "latest").await?;

            // yarn set version 3.x.x
            exec_command(
                self.get_bin_path(),
                vec!["set", "version", &self.config.version],
                &toolchain.root_dir,
            )
            .await?
        }

        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        &self.bin_path
    }

    fn get_download_path(&self) -> Option<&PathBuf> {
        None
    }

    fn get_install_dir(&self) -> &PathBuf {
        &self.install_dir
    }

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        Ok(get_bin_version(self.get_bin_path()).await?)
    }
}

#[async_trait]
impl PackageManager for YarnTool {
    async fn install_deps(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let mut args = vec!["install"];

        if self.is_v1() {
            args.push("--frozen-lockfile");
            args.push("--non-interactive");

            if is_ci() {
                args.push("--check-files");
            }
        } else {
            args.push("--immutable");

            if is_ci() {
                args.push("--check-cache");
            }
        }

        Ok(exec_command(self.get_bin_path(), args, &toolchain.root_dir).await?)
    }
}
