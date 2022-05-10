use crate::errors::ToolchainError;
use crate::helpers::{get_bin_name_suffix, get_bin_version};
use crate::traits::{Executable, Installable, Lifecycle, Logable, PackageManager};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::PnpmConfig;
use moon_logger::{color, debug};
use moon_utils::is_ci;
use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct PnpmTool {
    bin_path: Option<PathBuf>,

    pub config: PnpmConfig,
}

impl PnpmTool {
    pub fn new(config: &PnpmConfig) -> Result<PnpmTool, ToolchainError> {
        Ok(PnpmTool {
            bin_path: None,
            config: config.to_owned(),
        })
    }
}

impl Lifecycle for PnpmTool {}

impl Logable for PnpmTool {
    fn get_log_target(&self) -> String {
        String::from("moon:toolchain:pnpm")
    }
}

#[async_trait]
impl Installable for PnpmTool {
    async fn get_install_dir(&self, toolchain: &Toolchain) -> Result<PathBuf, ToolchainError> {
        toolchain.get_node().get_install_dir(toolchain).await
    }

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        get_bin_version(self.get_bin_path()).await
    }

    async fn is_installed(
        &self,
        toolchain: &Toolchain,
        check_version: bool,
    ) -> Result<bool, ToolchainError> {
        let target = self.get_log_target();

        if !toolchain
            .get_node()
            .get_npm()
            .is_global_dep_installed("pnpm")
            .await?
        {
            debug!(
                target: &target,
                "Package is not installed, attempting to install",
            );

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

    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let target = self.get_log_target();
        let node = toolchain.get_node();
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

            npm.install_global_dep("pnpm", self.config.version.as_str())
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Executable for PnpmTool {
    async fn find_bin_path(&mut self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let suffix = get_bin_name_suffix("pnpm", "cmd", false);
        let mut bin_path = self.get_install_dir(toolchain).await?.join(&suffix);

        // If bin doesn't exist in the install dir, try the global dir
        if !bin_path.exists() {
            bin_path = toolchain
                .get_node()
                .get_npm()
                .get_global_dir()
                .await?
                .join(&suffix);
        }

        self.bin_path = Some(bin_path);

        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        self.bin_path.as_ref().unwrap()
    }
}

#[async_trait]
impl PackageManager for PnpmTool {
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
