use crate::errors::ToolchainError;
use crate::helpers::{download_file_from_url, unpack};
use crate::tools::node::NodeTool;
use crate::traits::{Executable, Installable, Lifecycle, PackageManager};
use crate::ToolchainPaths;
use async_trait::async_trait;
use moon_config::YarnConfig;
use moon_lang::LockfileDependencyVersions;
use moon_logger::{color, debug, Logable};
use moon_node_lang::{node, yarn, YARN};
use moon_utils::process::Command;
use moon_utils::{fs, get_workspace_root, is_ci};
use rustc_hash::FxHashMap;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct YarnTool {
    bin_path: PathBuf,

    pub config: YarnConfig,

    download_path: PathBuf,

    download_version: String,

    install_dir: PathBuf,

    log_target: String,
}

impl YarnTool {
    pub fn new(
        paths: &ToolchainPaths,
        config: &Option<YarnConfig>,
    ) -> Result<YarnTool, ToolchainError> {
        let config = config.to_owned().unwrap_or_default();
        let install_dir = paths.tools.join("yarn").join(&config.version);

        // Yarn is installed through npm, but only v1 exists in the npm registry,
        // even if a consumer is using Yarn 2/3. https://www.npmjs.com/package/yarn
        // Yarn >= 2 work differently than normal packages, as their runtime code
        // is stored *within* the repository, and the v1 package detects it.
        // Because of this, we need to always install the v1 package!
        let download_version = if config.version.starts_with('1') {
            config.version.to_owned()
        } else {
            "1.22.19".to_owned()
        };

        Ok(YarnTool {
            bin_path: install_dir.join("bin/yarn.js"),
            download_path: paths
                .temp
                .join("yarn")
                .join(node::get_package_download_file("yarn", &download_version)),
            download_version,
            install_dir,
            log_target: String::from("moon:toolchain:yarn"),
            config,
        })
    }

    fn is_v1(&self) -> bool {
        self.config.version.starts_with('1')
    }
}

impl Logable for YarnTool {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }
}

#[async_trait]
impl Lifecycle<NodeTool> for YarnTool {
    async fn setup(&mut self, node: &NodeTool, check_version: bool) -> Result<u8, ToolchainError> {
        if !check_version || self.is_v1() {
            return Ok(0);
        }

        // We also dont want to *always* run this, so only run it when
        // we detect different yarn version files in the repo. Also note, we don't
        // have access to the workspace root here...
        let root = match env::var("MOON_WORKSPACE_ROOT") {
            Ok(root) => PathBuf::from(root),
            Err(_) => env::current_dir().unwrap_or_default(),
        };

        let yarn_bin = root
            .join(".yarn/releases")
            .join(format!("yarn-{}.cjs", self.config.version));

        if !yarn_bin.exists() {
            // We must do this here instead of `install`, because the bin path
            // isn't available yet during installation, only after!
            debug!(
                target: self.get_log_target(),
                "Updating package manager version with {}",
                color::shell(format!("yarn set version {}", self.config.version))
            );

            self.create_command(node)
                .args(["set", "version", &self.config.version])
                .exec_capture_output()
                .await?;

            if let Some(plugins) = &self.config.plugins {
                for plugin in plugins {
                    self.create_command(node)
                        .args(["plugin", "import", plugin])
                        .exec_capture_output()
                        .await?;
                }
            }
        }

        Ok(1)
    }
}

#[async_trait]
impl Installable<NodeTool> for YarnTool {
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
        if self.download_version == self.config.version {
            debug!(
                target: self.get_log_target(),
                "Installing yarn v{}", self.config.version
            );
        } else {
            debug!(
                target: self.get_log_target(),
                "Installing yarn v{} (via v{})", self.config.version, self.download_version
            );
        }

        if !self.download_path.exists() {
            download_file_from_url(
                node::get_npm_registry_url(
                    "yarn",
                    node::get_package_download_file("yarn", &self.download_version),
                ),
                &self.download_path,
            )
            .await?;
        }

        unpack(
            &self.download_path,
            &self.install_dir,
            &format!("yarn-v{}", self.download_version),
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Executable<NodeTool> for YarnTool {
    async fn find_bin_path(&mut self, _node: &NodeTool) -> Result<(), ToolchainError> {
        // If the global has moved, be sure to reference it
        // let bin_path = node::find_package_manager_bin(node.get_npm().get_global_dir()?, "yarn");

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
impl PackageManager<NodeTool> for YarnTool {
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
        // Yarn v1 doesnt dedupe natively, so use:
        // npx yarn-deduplicate yarn.lock
        if !self.is_v1() {
            self.create_command(node)
                .arg("dedupe")
                .cwd(working_dir)
                .log_running_command(log)
                .exec_capture_output()
                .await?;
        }

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(YARN.lock_filename)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(YARN.manifest_filename)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> Result<LockfileDependencyVersions, ToolchainError> {
        let Some(lockfile_path) = fs::find_upwards(YARN.lock_filename, project_root) else {
            return Ok(FxHashMap::default());
        };

        Ok(yarn::load_lockfile_dependencies(lockfile_path).await?)
    }

    async fn install_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolchainError> {
        let mut args = vec!["install"];

        if self.is_v1() {
            args.push("--ignore-engines");
        }

        if is_ci() {
            if self.is_v1() {
                args.push("--check-files");
                args.push("--frozen-lockfile");
                args.push("--non-interactive");
            } else {
                args.push("--immutable");
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

        if self.is_v1() {
            cmd.arg("install");
        } else {
            cmd.args(["workspaces", "focus"]);
            cmd.args(packages);

            let workspace_plugin =
                get_workspace_root().join(".yarn/plugins/@yarnpkg/plugin-workspace-tools.cjs");

            if !workspace_plugin.exists() {
                return Err(ToolchainError::RequiresYarnWorkspacesPlugin);
            }
        };

        if production_only {
            cmd.arg("--production");
        }

        cmd.exec_stream_output().await?;

        Ok(())
    }
}
