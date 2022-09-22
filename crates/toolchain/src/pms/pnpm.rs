use crate::errors::ToolchainError;
use crate::helpers::get_bin_version;
use crate::tools::node::NodeTool;
use crate::traits::{Executable, Installable, Lifecycle, PackageManager};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::PnpmConfig;
use moon_lang::LockfileDependencyVersions;
use moon_lang_node::{node, pnpm, PNPM};
use moon_logger::{color, debug, Logable};
use moon_utils::{fs, is_ci};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

pub struct PnpmTool {
    bin_path: PathBuf,

    pub config: PnpmConfig,

    install_dir: PathBuf,

    log_target: String,
}

impl PnpmTool {
    pub fn new(node: &NodeTool, config: &Option<PnpmConfig>) -> Result<PnpmTool, ToolchainError> {
        let install_dir = node.get_install_dir()?.clone();

        Ok(PnpmTool {
            bin_path: node::find_package_manager_bin(&install_dir, "pnpm"),
            config: config.to_owned().unwrap_or_default(),
            install_dir,
            log_target: String::from("moon:toolchain:pnpm"),
        })
    }
}

impl Logable for PnpmTool {
    fn get_log_target(&self) -> &str {
        &self.log_target
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
        if !self.is_executable()
            || (!node.is_corepack_aware()
                && !node.get_npm().is_global_dep_installed("pnpm").await?)
        {
            return Ok(false);
        }

        if !check_version {
            return Ok(true);
        }

        let log_target = self.get_log_target();
        let version = self.get_installed_version().await?;

        if version != self.config.version {
            debug!(
                target: log_target,
                "Package is on the wrong version ({}), attempting to reinstall", version
            );

            return Ok(false);
        }

        debug!(
            target: log_target,
            "Package has already been installed and is on the correct version",
        );

        Ok(true)
    }

    async fn install(&self, node: &NodeTool) -> Result<(), ToolchainError> {
        let log_target = self.get_log_target();
        let npm = node.get_npm();
        let package = format!("pnpm@{}", self.config.version);

        if node.is_corepack_aware() {
            debug!(
                target: log_target,
                "Enabling package manager with {}",
                color::shell(format!("corepack prepare {} --activate", package))
            );

            node.exec_corepack(["prepare", &package, "--activate"])
                .await?;
        } else {
            debug!(
                target: log_target,
                "Installing package manager with {}",
                color::shell(format!("npm install -g {}", package))
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
        let bin_path = node::find_package_manager_bin(node.get_npm().get_global_dir()?, "pnpm");

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

    fn get_lock_filename(&self) -> String {
        String::from(PNPM.lock_filenames[0])
    }

    fn get_manifest_filename(&self) -> String {
        String::from(PNPM.manifest_filename)
    }

    async fn get_resolved_depenencies(
        &self,
        project_root: &Path,
    ) -> Result<LockfileDependencyVersions, ToolchainError> {
        let lockfile_path = match fs::find_upwards(PNPM.lock_filenames[0], project_root) {
            Some(path) => path,
            None => {
                return Ok(HashMap::new());
            }
        };

        Ok(pnpm::load_lockfile_dependencies(lockfile_path)?)
    }

    async fn install_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let mut args = vec!["install"];
        let lockfile = toolchain.workspace_root.join(self.get_lock_filename());

        if is_ci() {
            // Will fail with "Headless installation requires a pnpm-lock.yaml file"
            if lockfile.exists() {
                args.push("--frozen-lockfile");
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

    async fn install_focused_dependencies(
        &self,
        toolchain: &Toolchain,
        package_names: &[String],
        production_only: bool,
    ) -> Result<(), ToolchainError> {
        let mut cmd = self.create_command();
        cmd.arg("install");

        if production_only {
            cmd.arg("--prod");
        }

        for package_name in package_names {
            cmd.arg(if production_only {
                "--filter-prod"
            } else {
                "--filter"
            });

            // https://pnpm.io/filtering#--filter-package_name-1
            cmd.arg(format!("{}...", package_name));
        }

        cmd.cwd(&toolchain.workspace_root)
            .exec_stream_output()
            .await?;

        Ok(())
    }
}
