use crate::errors::ToolchainError;
use crate::helpers::{download_file_from_url, unpack};
use crate::tools::node::NodeTool;
use crate::traits::{Executable, Installable, Lifecycle, PackageManager};
use crate::ToolchainPaths;
use async_trait::async_trait;
use moon_config::PnpmConfig;
use moon_lang::LockfileDependencyVersions;
use moon_logger::{debug, Logable};
use moon_node_lang::{node, pnpm, PNPM};
use moon_utils::process::Command;
use moon_utils::{fs, is_ci};
use rustc_hash::FxHashMap;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PnpmTool {
    bin_path: PathBuf,

    pub config: PnpmConfig,

    download_path: PathBuf,

    install_dir: PathBuf,

    log_target: String,
}

impl PnpmTool {
    pub fn new(
        paths: &ToolchainPaths,
        config: &Option<PnpmConfig>,
    ) -> Result<PnpmTool, ToolchainError> {
        let config = config.to_owned().unwrap_or_default();
        let install_dir = paths.tools.join("pnpm").join(&config.version);

        Ok(PnpmTool {
            bin_path: install_dir.join("bin/pnpm.cjs"),
            download_path: paths
                .temp
                .join("pnpm")
                .join(node::get_package_download_file("pnpm", &config.version)),
            install_dir,
            log_target: String::from("moon:toolchain:pnpm"),
            config,
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

    async fn is_installed(
        &self,
        _node: &NodeTool,
        _check_version: bool,
    ) -> Result<bool, ToolchainError> {
        Ok(self.bin_path.exists())
    }

    async fn install(&self, _node: &NodeTool) -> Result<(), ToolchainError> {
        debug!(
            target: self.get_log_target(),
            "Installing pnpm v{}", self.config.version
        );

        if !self.download_path.exists() {
            download_file_from_url(
                node::get_npm_registry_url(
                    "pnpm",
                    node::get_package_download_file("pnpm", &self.config.version),
                ),
                &self.download_path,
            )
            .await?;
        }

        unpack(&self.download_path, &self.install_dir, "package").await?;

        Ok(())
    }
}

#[async_trait]
impl Executable<NodeTool> for PnpmTool {
    async fn find_bin_path(&mut self, _node: &NodeTool) -> Result<(), ToolchainError> {
        // If the global has moved, be sure to reference it
        // let bin_path = node::find_package_manager_bin(node.get_npm().get_global_dir()?, "pnpm");

        // if bin_path.exists() {
        //     self.bin_path = bin_path;
        // }

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
    fn create_command(&self, node: &NodeTool) -> Command {
        let mut cmd = Command::new(node.get_bin_path());
        cmd.arg(self.get_bin_path());
        // cmd.env("PATH", get_path_env_var(bin_path.parent().unwrap()));
        cmd
    }

    async fn dedupe_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolchainError> {
        // pnpm doesn't support deduping, but maybe prune is good here?
        // https://pnpm.io/cli/prune
        self.create_command(node)
            .arg("prune")
            .cwd(working_dir)
            .log_running_command(log)
            .exec_capture_output()
            .await?;

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(PNPM.lock_filename)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(PNPM.manifest_filename)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> Result<LockfileDependencyVersions, ToolchainError> {
        let Some(lockfile_path) = fs::find_upwards(PNPM.lock_filename, project_root) else {
            return Ok(FxHashMap::default());
        };

        Ok(pnpm::load_lockfile_dependencies(lockfile_path)?)
    }

    async fn install_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolchainError> {
        let mut args = vec!["install"];

        if is_ci() {
            let lockfile = working_dir.join(self.get_lock_filename());

            // Will fail with "Headless installation requires a pnpm-lock.yaml file"
            if lockfile.exists() {
                args.push("--frozen-lockfile");
            }
        }

        let mut cmd = self.create_command(node);

        cmd.args(args).cwd(working_dir).log_running_command(log);

        if env::var("MOON_TEST_HIDE_INSTALL_OUTPUT").is_ok() {
            cmd.exec_capture_output().await?;
        } else {
            cmd.exec_stream_output().await?;
        }

        Ok(())
    }

    async fn install_focused_dependencies(
        &self,
        node: &NodeTool,
        packages: &[String],
        production_only: bool,
    ) -> Result<(), ToolchainError> {
        let mut cmd = self.create_command(node);
        cmd.arg("install");

        if production_only {
            cmd.arg("--prod");
        }

        for package in packages {
            cmd.arg(if production_only {
                "--filter-prod"
            } else {
                "--filter"
            });

            // https://pnpm.io/filtering#--filter-package_name-1
            cmd.arg(format!("{}...", package));
        }

        cmd.exec_stream_output().await?;

        Ok(())
    }
}
