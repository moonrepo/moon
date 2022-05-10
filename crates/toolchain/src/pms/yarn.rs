use crate::errors::ToolchainError;
use crate::helpers::{get_bin_name_suffix, get_bin_version};
use crate::tool::{Executable, Installable, PackageManager};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::YarnConfig;
use moon_logger::{color, debug};
use moon_utils::is_ci;
use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct YarnTool {
    bin_path: Option<PathBuf>,

    pub config: YarnConfig,
}

impl YarnTool {
    pub fn new(config: &YarnConfig) -> Result<YarnTool, ToolchainError> {
        Ok(YarnTool {
            bin_path: None,
            config: config.to_owned(),
        })
    }

    fn is_v1(&self) -> bool {
        self.config.version.starts_with('1')
    }
}

#[async_trait]
impl Installable for YarnTool {
    async fn get_install_dir(&self, toolchain: &Toolchain) -> Result<PathBuf, ToolchainError> {
        toolchain.get_node().get_install_dir(toolchain).await
    }

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        get_bin_version(&self.get_bin_path()).await
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
            .is_global_dep_installed("yarn")
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

    // Yarn is installed through npm, but only v1 exists in the npm registry,
    // even if a consumer is using Yarn 2/3. https://www.npmjs.com/package/yarn
    // Yarn >= 2 work differently than normal packages, as their runtime code
    // is stored *within* the repository, and the v1 package detects it.
    // Because of this, we need to always install the v1 package!
    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let target = self.get_log_target();
        let node = toolchain.get_node();
        let npm = node.get_npm();

        if self.is_v1() {
            let package = format!("yarn@{}", self.config.version);

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
                    "Installing package with {}",
                    color::shell(&format!("npm install -g {}", package))
                );

                npm.install_global_dep("yarn", &self.config.version).await?;
            }
        } else {
            if node.is_corepack_aware() {
                debug!(
                    target: &target,
                    "Enabling package manager with {}",
                    color::shell("corepack prepare yarn --activate")
                );

                node.exec_corepack(["prepare", "yarn", "--activate"])
                    .await?;
            } else {
                debug!(
                    target: &target,
                    "Installing legacy package with {}",
                    color::shell("npm install -g yarn@latest")
                );

                npm.install_global_dep("yarn", "latest").await?;
            }

            debug!(
                target: &target,
                "Installing package manager with {}",
                color::shell(&format!("yarn set version {}", self.config.version))
            );

            self.create_command()
                .args(["set", "version", &self.config.version])
                .cwd(&toolchain.workspace_root)
                .exec_capture_output()
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Executable for YarnTool {
    async fn find_bin_path(&mut self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let suffix = get_bin_name_suffix("yarn", "cmd", false);
        let mut bin_path = self.get_install_dir(toolchain).await?.join(suffix);

        // If bin doesn't exist in the install dir, try the global dir
        if !bin_path.exists() {
            bin_path = toolchain
                .get_node()
                .get_npm()
                .get_global_dir()
                .await?
                .join(suffix);
        }

        self.bin_path = Some(bin_path);

        Ok(())
    }

    fn get_bin_path(&self) -> PathBuf {
        self.bin_path.unwrap()
    }
}

#[async_trait]
impl PackageManager for YarnTool {
    async fn dedupe_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        // Yarn v1 doesnt dedupe natively, so use:
        // npx yarn-deduplicate yarn.lock
        if self.is_v1() {
            if toolchain
                .workspace_root
                .join(self.get_lockfile_name())
                .exists()
            {
                // Will error if the lockfile does not exist!
                toolchain
                    .get_node()
                    .get_npm()
                    .exec_package(
                        toolchain,
                        "yarn-deduplicate",
                        vec!["yarn-deduplicate", "yarn.lock"],
                    )
                    .await?;
            }

        // yarn dedupe
        } else {
            self.create_command()
                .arg("dedupe")
                .cwd(&toolchain.workspace_root)
                .exec_capture_output()
                .await?;
        }

        Ok(())
    }

    async fn exec_package(
        &self,
        toolchain: &Toolchain,
        package: &str,
        args: Vec<&str>,
    ) -> Result<(), ToolchainError> {
        // https://yarnpkg.com/cli/dlx
        let mut exec_args = vec!["dlx", "--package", package];
        exec_args.extend(args);

        self.create_command()
            .args(exec_args)
            .cwd(&toolchain.workspace_root)
            .exec_stream_output()
            .await?;

        Ok(())
    }

    fn get_lockfile_name(&self) -> String {
        String::from("yarn.lock")
    }

    fn get_log_target(&self) -> String {
        String::from("moon:toolchain:yarn")
    }

    fn get_workspace_dependency_range(&self) -> String {
        if self.is_v1() {
            String::from("*")
        } else {
            // https://yarnpkg.com/features/workspaces/#workspace-ranges-workspace
            String::from("workspace:*")
        }
    }

    async fn install_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let mut args = vec!["install"];

        if is_ci() {
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
