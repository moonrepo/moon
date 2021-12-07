use crate::errors::ToolchainError;
use crate::tool::Tool;
use crate::Toolchain;
use async_trait::async_trait;
use monolith_config::workspace::YarnConfig;
use std::env::consts;
use std::path::PathBuf;

#[derive(Debug)]
pub struct YarnTool {
    bin_path: PathBuf,

    install_dir: PathBuf,

    pub version: String,
}

impl YarnTool {
    pub fn load(
        toolchain: &Toolchain,
        config: &Option<YarnConfig>,
    ) -> Result<YarnTool, ToolchainError> {
        let node_tool = toolchain.get_node_tool();
        let install_dir = node_tool.get_install_dir().clone();
        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("yarn");
        } else {
            bin_path.push("bin/yarn");
        }

        let version = match config {
            Some(cfg) => cfg.version.clone(),
            None => "latest".to_owned(),
        };

        Ok(YarnTool {
            bin_path,
            install_dir,
            version,
        })
    }
}

#[async_trait]
impl Tool for YarnTool {
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
        // Yarn is installed through npm, but only v1 exists in the npm registry,
        // even if a consumer is using Yarn 2/3. https://www.npmjs.com/package/yarn
        // Yarn >= 2 work differently than normal packages, as their runtime code
        // is stored *within* the repository, and the v1 package detects it.
        // Because of this, we need to always install the v1 package!
        let version = if self.version.starts_with('1') {
            self.version.as_str()
        } else {
            "latest"
        };

        Ok(toolchain
            .get_npm_tool()
            .install_global_dep("yarn", version)
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
