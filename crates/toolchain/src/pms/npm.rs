use crate::errors::ToolchainError;
use crate::helpers::{get_bin_version, get_path_env_var};
use crate::tools::node::NodeTool;
use crate::traits::{Executable, Installable, Lifecycle, PackageManager};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::NpmConfig;
use moon_lang_node::{node, NPM};
use moon_logger::{color, debug, Logable};
use moon_utils::is_ci;
use moon_utils::process::{output_to_trimmed_string, Command};
use std::env;
use std::path::{Path, PathBuf};

pub struct NpmTool {
    bin_path: PathBuf,

    pub config: NpmConfig,

    global_install_dir: Option<PathBuf>,

    install_dir: PathBuf,

    log_target: String,
}

impl NpmTool {
    pub fn new(node: &NodeTool, config: &NpmConfig) -> Result<NpmTool, ToolchainError> {
        let install_dir = node.get_install_dir()?.clone();

        Ok(NpmTool {
            bin_path: node::find_package_manager_bin(&install_dir, "npm"),
            config: config.to_owned(),
            global_install_dir: None,
            install_dir,
            log_target: String::from("moon:toolchain:npm"),
        })
    }

    pub fn get_global_dir(&self) -> Result<&PathBuf, ToolchainError> {
        Ok(self
            .global_install_dir
            .as_ref()
            .unwrap_or(&self.install_dir))
    }

    pub async fn install_global_dep(
        &self,
        package: &str,
        version: &str,
    ) -> Result<(), ToolchainError> {
        self.create_command()
            .args(["install", "-g", &format!("{}@{}", package, version)])
            .exec_capture_output()
            .await?;

        Ok(())
    }

    pub async fn is_global_dep_installed(&self, package: &str) -> Result<bool, ToolchainError> {
        let output = self
            .create_command()
            .args(["list", "-g", package])
            .no_error_on_failure()
            .exec_capture_output()
            .await?;

        Ok(output.status.success())
    }
}

impl Logable for NpmTool {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }
}

#[async_trait]
impl Lifecycle<NodeTool> for NpmTool {
    async fn setup(&mut self, _node: &NodeTool, check_version: bool) -> Result<u8, ToolchainError> {
        if check_version {
            let output = self
                .create_command()
                .args(["config", "get", "prefix"])
                .exec_capture_output()
                .await?;

            self.global_install_dir = Some(PathBuf::from(output_to_trimmed_string(&output.stdout)));
        }

        Ok(0)
    }
}

#[async_trait]
impl Installable<NodeTool> for NpmTool {
    fn get_install_dir(&self) -> Result<&PathBuf, ToolchainError> {
        Ok(&self.install_dir)
    }

    async fn get_installed_version(&self) -> Result<String, ToolchainError> {
        get_bin_version(self.get_bin_path()).await
    }

    async fn is_installed(
        &self,
        _node: &NodeTool,
        check_version: bool,
    ) -> Result<bool, ToolchainError> {
        let log_target = self.get_log_target();

        if !self.is_executable() {
            return Ok(false);
        }

        if !check_version {
            return Ok(true);
        }

        let version = self.get_installed_version().await?;

        if self.config.version == "inherit" {
            debug!(
                target: log_target,
                "Using the version ({}) that came bundled with Node.js", version
            );

            return Ok(true);
        }

        if version == self.config.version {
            debug!(
                target: log_target,
                "Package has already been installed and is on the correct version",
            );

            return Ok(true);
        }

        debug!(
            target: log_target,
            "Package is on the wrong version ({}), attempting to reinstall", version
        );

        Ok(false)
    }

    async fn install(&self, node: &NodeTool) -> Result<(), ToolchainError> {
        if self.config.version == "inherit" {
            return Ok(());
        }

        let log_target = self.get_log_target();
        let package = format!("npm@{}", self.config.version);

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

            self.install_global_dep("npm", &self.config.version).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Executable<NodeTool> for NpmTool {
    async fn find_bin_path(&mut self, _node: &NodeTool) -> Result<(), ToolchainError> {
        // If the global has moved, be sure to reference it
        let bin_path = node::find_package_manager_bin(self.get_global_dir()?, "npm");

        if bin_path.exists() {
            self.bin_path = bin_path;
        }

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
    async fn dedupe_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        if !is_ci() {
            self.create_command()
                .args(["dedupe"])
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
        let mut exec_args = vec!["--silent", "--package", package, "--"];

        exec_args.extend(args);

        let install_dir = toolchain.get_node().get_install_dir()?;
        let npx_path = node::find_package_manager_bin(install_dir, "npx");

        Command::new(&npx_path)
            .args(exec_args)
            .cwd(&toolchain.workspace_root)
            .env("PATH", get_path_env_var(install_dir))
            .exec_stream_output()
            .await?;

        Ok(())
    }

    async fn find_package_bin(
        &self,
        toolchain: &Toolchain,
        starting_dir: &Path,
        bin_name: &str,
    ) -> Result<PathBuf, ToolchainError> {
        // npm binaries are symlinks to actual JavaScript files
        toolchain
            .get_node()
            .find_package_bin(starting_dir, bin_name)
    }

    fn get_lock_filename(&self) -> String {
        String::from(NPM.lock_filenames[0])
    }

    fn get_manifest_filename(&self) -> String {
        String::from(NPM.manifest_filename)
    }

    fn get_workspace_dependency_range(&self) -> String {
        String::from("*") // Doesn't support "workspace:*"
    }

    async fn install_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let mut args = vec!["install"];

        if is_ci() {
            let lockfile = toolchain.workspace_root.join(self.get_lock_filename());

            // npm will error if using `ci` and a lockfile does not exist!
            if lockfile.exists() {
                args.clear();
                args.push("ci");
            }
        } else {
            args.push("--no-audit");
        }

        args.push("--no-fund");

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
