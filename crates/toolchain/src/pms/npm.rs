use crate::errors::ToolchainError;
use crate::helpers::{get_bin_name_suffix, get_bin_version, get_path_env_var};
use crate::tools::node::NodeTool;
use crate::traits::{Executable, Installable, Lifecycle, Logable, PackageManager};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::NpmConfig;
use moon_logger::{color, debug};
use moon_utils::is_ci;
use moon_utils::process::{output_to_trimmed_string, Command};
use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct NpmTool {
    bin_path: Option<PathBuf>,

    pub config: NpmConfig,
}

impl NpmTool {
    pub fn new(config: &NpmConfig) -> Result<NpmTool, ToolchainError> {
        Ok(NpmTool {
            bin_path: None,
            config: config.to_owned(),
        })
    }

    pub async fn get_global_dir(&self) -> Result<PathBuf, ToolchainError> {
        let output = self
            .create_command()
            .args(["config", "get", "prefix"])
            .exec_capture_output()
            .await?;
        let dir = output_to_trimmed_string(&output.stdout);

        Ok(PathBuf::from(dir))
    }

    pub async fn install_global_dep(
        &self,
        package: &str,
        version: &str,
    ) -> Result<(), ToolchainError> {
        self.create_command()
            .args(["install", "-g", &format!("{}@{}", package, version)])
            .exec_stream_output()
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

impl Lifecycle for NpmTool {}

impl Logable for NpmTool {
    fn get_log_target(&self) -> String {
        String::from("moon:toolchain:npm")
    }
}

#[async_trait]
impl Installable<NodeTool> for NpmTool {
    async fn get_install_dir(&self, node: &NodeTool) -> Result<PathBuf, ToolchainError> {
        node.get_install_dir(toolchain).await
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

        if !self.is_executable() {
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

        if self.config.version == "inherit" {
            debug!(
                target: &target,
                "Using the version ({}) that came bundled with Node.js", version
            );

            return Ok(true);
        }

        if version == self.config.version {
            debug!(
                target: &target,
                "Package has already been installed and is on the correct version",
            );

            return Ok(true);
        }

        debug!(
            target: &target,
            "Package is on the wrong version ({}), attempting to reinstall", version
        );

        Ok(false)
    }

    async fn install(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        if self.config.version == "inherit" {
            return Ok(());
        }

        let target = self.get_log_target();
        let node = toolchain.get_node();
        let package = format!("npm@{}", self.config.version);

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

            self.install_global_dep("npm", &self.config.version).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Executable for NpmTool {
    async fn find_bin_path(&mut self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let bin_path = self
            .get_install_dir(toolchain)
            .await?
            .join(get_bin_name_suffix("npm", "cmd", false));

        self.bin_path = Some(bin_path);

        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        self.bin_path.as_ref().unwrap()
    }

    fn is_executable(&self) -> bool {
        self.bin_path.is_some()
    }
}

#[async_trait]
impl PackageManager for NpmTool {
    async fn dedupe_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        self.create_command()
            .args(["dedupe"])
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
        let mut exec_args = vec!["--silent", "--package", package, "--"];

        exec_args.extend(args);

        let bin_dir = toolchain.get_node().get_install_dir(toolchain).await?;
        let npx_path = bin_dir.join(get_bin_name_suffix("npx", "exe", false));

        Command::new(&npx_path)
            .args(exec_args)
            .cwd(&toolchain.workspace_root)
            .env("PATH", get_path_env_var(bin_dir))
            .exec_stream_output()
            .await?;

        Ok(())
    }

    fn get_lockfile_name(&self) -> String {
        String::from("package-lock.json")
    }

    fn get_workspace_dependency_range(&self) -> String {
        String::from("*") // Doesn't support "workspace:*"
    }

    async fn install_dependencies(&self, toolchain: &Toolchain) -> Result<(), ToolchainError> {
        let mut args = vec!["install"];

        if is_ci() {
            let lockfile = toolchain.workspace_root.join(self.get_lockfile_name());

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
