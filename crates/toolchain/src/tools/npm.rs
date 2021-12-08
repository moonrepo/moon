use crate::errors::ToolchainError;
use crate::helpers::{exec_command, get_bin_version, is_ci};
use crate::tool::{PackageManager, Tool};
use crate::Toolchain;
use async_trait::async_trait;
use monolith_config::workspace::NpmConfig;
use std::env::consts;
use std::path::PathBuf;

#[derive(Debug)]
pub struct NpmTool {
    bin_path: PathBuf,

    install_dir: PathBuf,

    pub config: NpmConfig,
}

impl NpmTool {
    pub fn new(toolchain: &Toolchain, config: &NpmConfig) -> Result<NpmTool, ToolchainError> {
        let node_tool = toolchain.get_node();
        let install_dir = node_tool.get_install_dir().clone();
        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("npm");
        } else {
            bin_path.push("bin/npm");
        }

        Ok(NpmTool {
            bin_path,
            config: config.to_owned(),
            install_dir,
        })
    }

    pub async fn add_global_dep(&self, name: &str, version: &str) -> Result<(), ToolchainError> {
        let package = format!("{}@{}", name, version);

        exec_command(
            self.get_bin_path(),
            vec!["install", "-g", package.as_str()],
            &self.install_dir,
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Tool for NpmTool {
    fn is_downloaded(&self) -> bool {
        true
    }

    async fn download(&self) -> Result<(), ToolchainError> {
        Ok(()) // This is handled by node
    }

    async fn is_installed(&self) -> Result<bool, ToolchainError> {
        Ok(self.bin_path.exists() && self.get_installed_version().await? == self.config.version)
    }

    async fn install(&self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
        // npm install -g npm
        self.add_global_dep("npm", self.config.version.as_str())
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
impl PackageManager for NpmTool {
    async fn install_deps(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        Ok(exec_command(
            self.get_bin_path(),
            vec![if is_ci() { "ci" } else { "install " }],
            &toolchain.root_dir,
        )
        .await?)
    }
}
