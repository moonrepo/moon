use crate::errors::ToolchainError;
use crate::helpers::{exec_command, get_bin_version};
use crate::tool::Tool;
use crate::Toolchain;
use async_trait::async_trait;
use moon_logger::{color, debug, trace};
use std::env::consts;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct NpxTool {
    bin_path: PathBuf,

    install_dir: PathBuf,
}

impl NpxTool {
    pub fn new(toolchain: &Toolchain) -> NpxTool {
        let install_dir = toolchain.get_node().get_install_dir().clone();
        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("npx");
        } else {
            bin_path.push("bin/npx");
        }

        debug!(
            target: "moon:toolchain:npx",
            "Creating tool at {}",
            color::file_path(&bin_path)
        );

        NpxTool {
            bin_path,
            install_dir,
        }
    }

    pub async fn exec(
        &self,
        package: &str,
        args: Vec<&str>,
        cwd: &Path,
    ) -> Result<(), ToolchainError> {
        let mut exec_args = vec!["--package", package, "--"];

        exec_args.extend(args);

        exec_command(self.get_bin_path(), exec_args, cwd).await?;

        Ok(())
    }
}

#[async_trait]
impl Tool for NpxTool {
    fn is_downloaded(&self) -> bool {
        true
    }

    async fn download(&self, _host: Option<&str>) -> Result<(), ToolchainError> {
        trace!(
            target: "moon:toolchain:npx",
            "No download required as it comes bundled with Node.js"
        );

        Ok(()) // This is handled by node
    }

    async fn is_installed(&self) -> Result<bool, ToolchainError> {
        debug!(
            target: "moon:toolchain:npx",
            "Package has already been installed and is on the correct version",
        );

        Ok(self.bin_path.exists())
    }

    async fn install(&self, _toolchain: &Toolchain) -> Result<(), ToolchainError> {
        Ok(()) // Comes pre-installed
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
