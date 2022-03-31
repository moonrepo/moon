use crate::errors::ToolchainError;
use crate::helpers::{get_bin_version, get_path_env_var};
use crate::tool::{PackageManager, Tool};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::PnpmConfig;
use moon_logger::{color, debug, trace};
use moon_utils::is_ci;
use moon_utils::process::{create_command, exec_command, Output};
use std::env::consts;
use std::path::PathBuf;

#[derive(Clone, Debug)]
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
            bin_path.push("pnpm.cmd");
        } else {
            bin_path.push("bin/pnpm");
        }

        debug!(
            target: "moon:toolchain:pnpm",
            "Creating tool at {}",
            color::path(&bin_path)
        );

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
        true
    }

    async fn download(&self, _host: Option<&str>) -> Result<(), ToolchainError> {
        trace!(
            target: "moon:toolchain:pnpm",
            "No download required as it comes bundled with Node.js"
        );

        Ok(()) // This is handled by node
    }

    async fn is_installed(&self, check_version: bool) -> Result<bool, ToolchainError> {
        if self.bin_path.exists() {
            if !check_version {
                return Ok(true);
            }

            let version = self.get_installed_version().await?;

            if version == self.config.version {
                debug!(
                    target: "moon:toolchain:pnpm",
                    "Package has already been installed and is on the correct version",
                );

                return Ok(true);
            }

            debug!(
                target: "moon:toolchain:pnpm",
                "Package is on the wrong version ({}), attempting to reinstall",
                version
            );
        }

        Ok(false)
    }

    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let package = format!("pnpm@{}", self.config.version);

        if toolchain.get_node().is_corepack_aware() {
            debug!(
                target: "moon:toolchain:pnpm",
                "Enabling package manager with {}",
                color::shell(&format!("corepack prepare {} --activate", package))
            );

            toolchain
                .get_node()
                .exec_corepack(["prepare", &package, "--activate"])
                .await?;
        } else {
            debug!(
                target: "moon:toolchain:pnpm",
                "Installing package manager with {}",
                color::shell(&format!("npm install -g {}", package))
            );

            toolchain
                .get_npm()
                .add_global_dep("pnpm", self.config.version.as_str())
                .await?;
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
impl PackageManager for PnpmTool {
    async fn dedupe_dependencies(&self, toolchain: &Toolchain) -> Result<Output, ToolchainError> {
        // pnpm doesn't support deduping, but maybe prune is good here?
        // https://pnpm.io/cli/prune
        Ok(exec_command(
            create_command(self.get_bin_path())
                .args(["prune"])
                .current_dir(&toolchain.workspace_root)
                .env("PATH", get_path_env_var(self.get_bin_dir())),
        )
        .await?)
    }

    async fn exec_package(
        &self,
        toolchain: &Toolchain,
        package: &str,
        args: Vec<&str>,
    ) -> Result<Output, ToolchainError> {
        let mut exec_args = vec!["--package", package, "dlx"];

        exec_args.extend(args);

        // https://pnpm.io/cli/dlx
        Ok(exec_command(
            create_command(self.get_bin_path())
                .args(exec_args)
                .current_dir(&toolchain.workspace_root)
                .env("PATH", get_path_env_var(self.get_bin_dir())),
        )
        .await?)
    }

    fn get_lockfile_name(&self) -> String {
        String::from("pnpm-lock.yaml")
    }

    fn get_workspace_dependency_range(&self) -> String {
        // https://pnpm.io/workspaces#workspace-protocol-workspace
        String::from("workspace:*")
    }

    async fn install_dependencies(&self, toolchain: &Toolchain) -> Result<Output, ToolchainError> {
        let mut args = vec!["install"];

        if is_ci() {
            args.push("--frozen-lockfile");
        }

        Ok(exec_command(
            create_command(self.get_bin_path())
                .args(args)
                .current_dir(&toolchain.workspace_root)
                .env("PATH", get_path_env_var(self.get_bin_dir())),
        )
        .await?)
    }
}
