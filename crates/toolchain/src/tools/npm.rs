use crate::errors::ToolchainError;
use crate::tool::Tool;
use crate::Toolchain;
use async_trait::async_trait;
use monolith_config::workspace::NpmConfig;
use std::env::consts;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug)]
pub struct NpmTool {
    bin_path: PathBuf,

    install_dir: PathBuf,

    pub version: String,
}

impl NpmTool {
    pub fn load(
        toolchain: &Toolchain,
        config: &Option<NpmConfig>,
    ) -> Result<NpmTool, ToolchainError> {
        let node_tool = toolchain.get_node_tool();
        let install_dir = node_tool.get_install_dir().clone();
        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("npm");
        } else {
            bin_path.push("bin/npm");
        }

        let version = match config {
            Some(cfg) => cfg.version.clone(),
            None => "latest".to_owned(),
        };

        Ok(NpmTool {
            bin_path,
            install_dir,
            version,
        })
    }

    pub async fn install_global_dep(
        &self,
        name: &str,
        version: &str,
    ) -> Result<(), ToolchainError> {
        let package = format!("{}@{}", name, version);

        let mut child = Command::new(self.get_bin_path())
            .args(["install", "-g", package.as_str()])
            .spawn()
            .map_err(|_| ToolchainError::FailedToInstall)?;

        child
            .wait()
            .await
            .map_err(|_| ToolchainError::FailedToInstall)?;

        Ok(())
    }
}

#[async_trait]
impl Tool for NpmTool {
    fn is_downloaded(&self) -> bool {
        false
    }

    async fn download(&self) -> Result<(), ToolchainError> {
        Ok(()) // This is handled by node
    }

    fn is_installed(&self) -> bool {
        self.bin_path.exists()
    }

    async fn install(&self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
        Ok(self
            .install_global_dep("npm", self.version.as_str())
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
