use crate::errors::ToolchainError;
use crate::helpers::exec_command;
use crate::tool::{PackageManager, Tool};
use crate::Toolchain;
use async_trait::async_trait;
use monolith_config::workspace::PnpmConfig;
use std::env::consts;
use std::path::PathBuf;

#[derive(Debug)]
pub struct PnpmTool {
    bin_path: PathBuf,

    install_dir: PathBuf,

    pub version: String,
}

impl PnpmTool {
    pub fn load(
        toolchain: &Toolchain,
        config: &Option<PnpmConfig>,
    ) -> Result<PnpmTool, ToolchainError> {
        let node_tool = toolchain.get_node_tool();
        let install_dir = node_tool.get_install_dir().clone();
        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("pnpm");
        } else {
            bin_path.push("bin/pnpm");
        }

        let version = match config {
            Some(cfg) => cfg.version.clone(),
            None => "latest".to_owned(),
        };

        Ok(PnpmTool {
            bin_path,
            install_dir,
            version,
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

    fn is_installed(&self) -> bool {
        self.bin_path.exists()
    }

    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        Ok(toolchain
            .get_npm_tool()
            .install_global_dep("pnpm", self.version.as_str())
            .await?)
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
}

#[async_trait]
impl PackageManager for PnpmTool {
    async fn install_deps(&self, root_dir: &PathBuf) -> Result<(), ToolchainError> {
        Ok(exec_command(self.get_bin_path(), vec!["install"], root_dir).await?)
    }
}
