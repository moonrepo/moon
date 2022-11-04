use crate::errors::ToolchainError;
use crate::helpers::{download_file_from_url, unpack};
use crate::tools::node::NodeTool;
use crate::traits::{Executable, Installable, Lifecycle, PackageManager};
use crate::ToolchainPaths;
use async_trait::async_trait;
use moon_config::NpmConfig;
use moon_lang::LockfileDependencyVersions;
use moon_logger::{debug, Logable};
use moon_node_lang::{node, npm, NPM};
use moon_utils::process::Command;
use moon_utils::{fs, is_ci};
use rustc_hash::FxHashMap;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct NpmTool {
    bin_path: PathBuf,

    pub config: NpmConfig,

    download_path: PathBuf,

    install_dir: PathBuf,

    log_target: String,
}

impl NpmTool {
    pub fn new(paths: &ToolchainPaths, config: &NpmConfig) -> Result<NpmTool, ToolchainError> {
        let install_dir = paths.tools.join("npm").join(&config.version);

        Ok(NpmTool {
            bin_path: install_dir.join("bin/npm-cli.js"),
            download_path: paths
                .temp
                .join("npm")
                .join(node::get_package_download_file("npm", &config.version)),
            install_dir,
            log_target: String::from("moon:toolchain:npm"),
            config: config.to_owned(),
        })
    }
}

impl Logable for NpmTool {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }
}

#[async_trait]
impl Lifecycle<NodeTool> for NpmTool {
    async fn setup(
        &mut self,
        _node: &NodeTool,
        _check_version: bool,
    ) -> Result<u8, ToolchainError> {
        Ok(0)
    }
}

#[async_trait]
impl Installable<NodeTool> for NpmTool {
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
            "Installing npm v{}", self.config.version
        );

        if !self.download_path.exists() {
            download_file_from_url(
                node::get_npm_registry_url(
                    "npm",
                    node::get_package_download_file("npm", &self.config.version),
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
impl Executable<NodeTool> for NpmTool {
    async fn find_bin_path(&mut self, _node: &NodeTool) -> Result<(), ToolchainError> {
        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        &self.bin_path
    }

    fn is_executable(&self) -> bool {
        true
    }
}

#[async_trait]
impl PackageManager<NodeTool> for NpmTool {
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
        self.create_command(node)
            .args(["dedupe"])
            .cwd(working_dir)
            .log_running_command(log)
            .exec_capture_output()
            .await?;

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(NPM.lock_filename)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(NPM.manifest_filename)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> Result<LockfileDependencyVersions, ToolchainError> {
        let Some(lockfile_path) = fs::find_upwards(NPM.lock_filename, project_root) else {
            return Ok(FxHashMap::default());
        };

        Ok(npm::load_lockfile_dependencies(lockfile_path)?)
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

            // npm will error if using `ci` and a lockfile does not exist!
            if lockfile.exists() {
                args.clear();
                args.push("ci");
            }
        } else {
            args.push("--no-audit");
        }

        args.push("--no-fund");

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
        package_names: &[String],
        production_only: bool,
    ) -> Result<(), ToolchainError> {
        let mut cmd = self.create_command(node);
        cmd.args(["install"]);

        if production_only {
            cmd.arg("--production");
        }

        for package_name in package_names {
            cmd.args(["--workspace", package_name]);
        }

        cmd.exec_stream_output().await?;

        Ok(())
    }
}
