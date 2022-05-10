use crate::errors::ToolchainError;
use crate::helpers::{get_bin_name_suffix, get_bin_version, get_path_env_var};
use crate::tool::{Executable, Installable, PackageManager};
use crate::Toolchain;
use async_trait::async_trait;
use moon_config::NpmConfig;
use moon_logger::{color, debug};
use moon_utils::is_ci;
use moon_utils::process::Command;
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

    pub async fn add_global_dep(&self, name: &str, version: &str) -> Result<(), ToolchainError> {
        let package = format!("{}@{}", name, version);

        self.create_command()
            .args(["install", "-g", &package])
            .exec_stream_output()
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Installable for NpmTool {
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
        if !check_version {
            return Ok(true);
        }

        let target = self.get_log_target();
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
        let package = format!("npm@{}", self.config.version);

        if toolchain.get_node().is_corepack_aware() {
            debug!(
                target: &target,
                "Enabling package manager with {}",
                color::shell(&format!("corepack prepare {} --activate", package))
            );

            toolchain
                .get_node()
                .exec_corepack(["prepare", &package, "--activate"])
                .await?;
        } else {
            debug!(
                target: &target,
                "Installing package manager with {}",
                color::shell(&format!("npm install -g {}", package))
            );

            self.add_global_dep("npm", &self.config.version).await?;
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

    fn get_bin_path(&self) -> PathBuf {
        self.bin_path.unwrap()
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

        let bin_dir = self.get_bin_path().parent().unwrap().to_path_buf();
        let npx_path = bin_dir.join(get_bin_name_suffix("corepack", "exe", false));

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

    fn get_log_target(&self) -> String {
        String::from("moon:toolchain:npm")
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
