use crate::errors::ToolchainError;
use crate::helpers::{exec_command, get_bin_version};
use crate::tool::{PackageManager, Tool};
use crate::Toolchain;
use async_trait::async_trait;
use monolith_config::workspace::PnpmConfig;
use std::env::consts;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PnpmTool {
    bin_path: PathBuf,

    install_dir: PathBuf,

    pub config: PnpmConfig,
}

impl PnpmTool {
    pub fn new(toolchain: &Toolchain, config: &PnpmConfig) -> Result<PnpmTool, ToolchainError> {
        let install_dir = toolchain.get_node().get_install_dir().clone();
        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("pnpm");
        } else {
            bin_path.push("bin/pnpm");
        }

        Ok(PnpmTool {
            bin_path,
            config: config.to_owned(),
            install_dir,
        })
    }
}

#[async_trait]
impl Tool for PnpmTool {
    fn is_downloaded(&self) -> bool {
        false
    }

    async fn download(&self) -> Result<(), ToolchainError> {
        Ok(()) // This is handled by node
    }

    async fn is_installed(&self) -> Result<bool, ToolchainError> {
        Ok(self.bin_path.exists() && self.get_installed_version().await? == self.config.version)
    }

    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        // npm install -g pnpm
        toolchain
            .get_npm()
            .add_global_dep("pnpm", self.config.version.as_str())
            .await?;

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
impl PackageManager for PnpmTool {
    async fn install_deps(&self, root_dir: &Path) -> Result<(), ToolchainError> {
        Ok(exec_command(self.get_bin_path(), vec!["install"], root_dir).await?)
    }
}
