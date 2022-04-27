use crate::errors::ToolchainError;
use crate::helpers::{get_bin_version, get_path_env_var};
use crate::tool::{PackageManager, Tool};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::YarnConfig;
use moon_logger::{color, debug, trace};
use moon_utils::is_ci;
use moon_utils::process::Command;
use std::env::{self, consts};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct YarnTool {
    bin_path: PathBuf,

    install_dir: PathBuf,

    pub config: YarnConfig,
}

impl YarnTool {
    pub fn new(toolchain: &Toolchain, config: &YarnConfig) -> Result<YarnTool, ToolchainError> {
        let install_dir = toolchain.get_node().get_install_dir().clone();
        let mut bin_path = install_dir.clone();

        if consts::OS == "windows" {
            bin_path.push("yarn.cmd");
        } else {
            bin_path.push("bin/yarn");
        }

        debug!(
            target: "moon:toolchain:yarn",
            "Creating tool at {}",
            color::path(&bin_path)
        );

        Ok(YarnTool {
            bin_path,
            config: config.to_owned(),
            install_dir,
        })
    }

    fn is_v1(&self) -> bool {
        self.config.version.starts_with('1')
    }
}

#[async_trait]
impl Tool for YarnTool {
    fn is_downloaded(&self) -> bool {
        true
    }

    async fn download(&self, _host: Option<&str>) -> Result<(), ToolchainError> {
        trace!(
            target: "moon:toolchain:yarn",
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
                    target: "moon:toolchain:yarn",
                    "Package has already been installed and is on the correct version",
                );

                return Ok(true);
            }

            debug!(
                target: "moon:toolchain:yarn",
                "Package is on the wrong version ({}), attempting to reinstall",
                version
            );
        }

        Ok(false)
    }

    // Yarn is installed through npm, but only v1 exists in the npm registry,
    // even if a consumer is using Yarn 2/3. https://www.npmjs.com/package/yarn
    // Yarn >= 2 work differently than normal packages, as their runtime code
    // is stored *within* the repository, and the v1 package detects it.
    // Because of this, we need to always install the v1 package!
    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let node = toolchain.get_node();
        let npm = toolchain.get_npm();

        if self.is_v1() {
            let package = format!("yarn@{}", self.config.version);

            if node.is_corepack_aware() {
                debug!(
                    target: "moon:toolchain:yarn",
                    "Enabling package manager with {}",
                    color::shell(&format!("corepack prepare {} --activate", package))
                );

                node.exec_corepack(["prepare", &package, "--activate"])
                    .await?;
            } else {
                debug!(
                    target: "moon:toolchain:yarn",
                    "Installing package with {}",
                    color::shell(&format!("npm install -g {}", package))
                );

                npm.add_global_dep("yarn", &self.config.version).await?;
            }
        } else {
            if node.is_corepack_aware() {
                debug!(
                    target: "moon:toolchain:yarn",
                    "Enabling package manager with {}",
                    color::shell("corepack prepare yarn --activate")
                );

                node.exec_corepack(["prepare", "yarn", "--activate"])
                    .await?;
            } else {
                debug!(
                    target: "moon:toolchain:yarn",
                    "Installing legacy package with {}",
                    color::shell("npm install -g yarn@latest")
                );

                npm.add_global_dep("yarn", "latest").await?;
            }

            debug!(
                target: "moon:toolchain:yarn",
                "Installing package manager with {}",
                color::shell(&format!("yarn set version {}", self.config.version))
            );

            Command::new(self.get_bin_path())
                .args(["set", "version", &self.config.version])
                .cwd(&toolchain.workspace_root)
                .env("PATH", get_path_env_var(self.get_bin_dir()))
                .exec_capture_output()
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
impl PackageManager for YarnTool {
    async fn dedupe_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        // Yarn v1 doesnt dedupe natively, so use:
        // npx yarn-deduplicate yarn.lock
        if self.is_v1() {
            toolchain
                .get_npm()
                .exec_package(
                    toolchain,
                    "yarn-deduplicate",
                    vec!["yarn-deduplicate", "yarn.lock"],
                )
                .await?;

        // yarn dedupe
        } else {
            Command::new(self.get_bin_path())
                .arg("dedupe")
                .cwd(&toolchain.workspace_root)
                .env("PATH", get_path_env_var(self.get_bin_dir()))
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
        let mut exec_args = vec!["dlx", "--package", package];

        exec_args.extend(args);

        // https://yarnpkg.com/cli/dlx
        Command::new(self.get_bin_path())
            .args(exec_args)
            .cwd(&toolchain.workspace_root)
            .env("PATH", get_path_env_var(self.get_bin_dir()))
            .exec_stream_output()
            .await?;

        Ok(())
    }

    fn get_lockfile_name(&self) -> String {
        String::from("yarn.lock")
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

        let mut process = Command::new(self.get_bin_path());

        process
            .args(args)
            .cwd(&toolchain.workspace_root)
            .env("PATH", get_path_env_var(self.get_bin_dir()));

        if env::var("MOON_TEST_HIDE_INSTALL_OUTPUT").is_ok() {
            process.exec_capture_output().await?;
        } else {
            process.exec_stream_output().await?;
        }

        Ok(())
    }
}
