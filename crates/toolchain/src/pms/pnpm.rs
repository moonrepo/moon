use crate::errors::ToolchainError;
use crate::helpers::{get_bin_name_suffix, get_bin_version};
use crate::tools::node::NodeTool;
use crate::traits::{Executable, Installable, Lifecycle, PackageManager};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::PnpmConfig;
use moon_logger::{color, debug, Logable};
use moon_utils::is_ci;
use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct PnpmTool {
    bin_path: PathBuf,

    pub config: PnpmConfig,

    install_dir: PathBuf,
}

impl PnpmTool {
    pub fn new(node: &NodeTool, config: &PnpmConfig) -> Result<PnpmTool, ToolchainError> {
        let install_dir = node.get_install_dir()?.clone();

        Ok(PnpmTool {
            bin_path: install_dir.join(get_bin_name_suffix("pnpm", "cmd", false)),
            config: config.to_owned(),
            install_dir,
        })
    }
}

impl Logable for PnpmTool {
    fn get_log_target(&self) -> String {
        String::from("moon:toolchain:pnpm")
    }
}

impl Lifecycle<NodeTool> for PnpmTool {}

#[async_trait]
impl Installable<NodeTool> for PnpmTool {
    fn get_install_dir(&self) -> Result<&PathBuf, ToolchainError> {
        Ok(&self.install_dir)
    }

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        get_bin_version(self.get_bin_path()).await
    }

    async fn is_installed(
        &self,
        node: &NodeTool,
        check_version: bool,
    ) -> Result<bool, ToolchainError> {
        let target = self.get_log_target();

        if !self.is_executable()
            || (!node.is_corepack_aware()
                && !node.get_npm().is_global_dep_installed("pnpm").await?)
        {
            return Ok(false);
        }

        if !check_version {
            return Ok(true);
        }

        let version = self.get_installed_version().await?;

        if version != self.config.version {
            debug!(
                target: &target,
                "Package is on the wrong version ({}), attempting to reinstall", version
            );

            return Ok(false);
        }

        debug!(
            target: &target,
            "Package has already been installed and is on the correct version",
        );

        Ok(true)
    }

    async fn install(&self, node: &NodeTool) -> Result<(), ToolchainError> {
        let target = self.get_log_target();
        let npm = node.get_npm();
        let package = format!("pnpm@{}", self.config.version);

        if node.is_corepack_aware() {
            debug!(
                target: &target,
                "Enabling package manager with {}",
                color::shell(&format!("corepack prepare {} --activate", package))
            );

            node.exec_corepack(["prepare", &package, "--activate"])
                .await?;
        } else {
            debug!(
                target: &target,
                "Installing package manager with {}",
                color::shell(&format!("npm install -g {}", package))
            );

            npm.install_global_dep("pnpm", &self.config.version).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Executable<NodeTool> for PnpmTool {
    async fn find_bin_path(&mut self, node: &NodeTool) -> Result<(), ToolchainError> {
        // If the global has moved, be sure to reference it
        let bin_path = node
            .get_npm()
            .get_global_dir()?
            .join(get_bin_name_suffix("pnpm", "cmd", false));

        if bin_path.exists() {
            self.bin_path = bin_path;
        }

        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        &self.bin_path
    }

    fn is_executable(&self) -> bool {
        self.bin_path.exists()
    }
}

#[async_trait]
impl PackageManager<NodeTool> for PnpmTool {
    async fn dedupe_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        // pnpm doesn't support deduping, but maybe prune is good here?
        // https://pnpm.io/cli/prune
        self.create_command()
            .arg("prune")
            .cwd(&toolchain.workspace_root)
            .exec_capture_output()
            .await?;

        Ok(())
    }

    async fn exec_package(
        &self,
        toolchain: &Toolchain,
        package: &str,
        args: Vec<&str>,
    ) -> Result<(), ToolchainError> {
        // https://pnpm.io/cli/dlx
        let mut exec_args = vec!["--package", package, "dlx"];
        exec_args.extend(args);

        self.create_command()
            .args(exec_args)
            .cwd(&toolchain.workspace_root)
            .exec_stream_output()
            .await?;

        Ok(())
    }

    fn get_lockfile_name(&self) -> String {
        String::from("pnpm-lock.yaml")
    }

    fn get_workspace_dependency_range(&self) -> String {
        // https://pnpm.io/workspaces#workspace-protocol-workspace
        String::from("workspace:*")
    }

    async fn install_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let mut args = vec!["install"];

        if is_ci() {
            args.push("--frozen-lockfile");
        }

        let mut cmd = self.create_command();

        cmd.args(args).cwd(&toolchain.workspace_root);

        if env::var("MOON_TEST_HIDE_INSTALL_OUTPUT").is_ok() {
            cmd.exec_capture_output().await?;
        } else {
            cmd.exec_stream_output().await?;
        }

        Ok(())
    }
}
