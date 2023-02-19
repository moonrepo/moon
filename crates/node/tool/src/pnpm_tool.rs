use crate::node_tool::NodeTool;
use moon_config::PnpmConfig;
use moon_logger::debug;
use moon_node_lang::{pnpm, LockfileDependencyVersions, PNPM};
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{get_path_env_var, DependencyManager, Tool, ToolError};
use moon_utils::process::Command;
use moon_utils::{fs, is_ci, semver};
use proto::{
    async_trait,
    node::{NodeDependencyManager, NodeDependencyManagerType},
    Describable, Executable, Installable, Proto, Shimable, Tool as ProtoTool,
};
use rustc_hash::FxHashMap;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PnpmTool {
    pub config: PnpmConfig,

    pub global: bool,

    pub tool: NodeDependencyManager,
}

impl PnpmTool {
    pub fn new(proto: &Proto, config: &Option<PnpmConfig>) -> Result<PnpmTool, ToolError> {
        let config = config.to_owned().unwrap_or_default();

        Ok(PnpmTool {
            global: config.version.is_none(),
            config,
            tool: NodeDependencyManager::new(proto, NodeDependencyManagerType::Pnpm),
        })
    }
}

#[async_trait]
impl Tool for PnpmTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<PathBuf, ToolError> {
        Ok(if self.global {
            "pnpm".into()
        } else {
            self.tool.get_bin_path()?.to_path_buf()
        })
    }

    fn get_shim_path(&self) -> Option<PathBuf> {
        self.tool.get_shim_path().map(|p| p.to_path_buf())
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        let mut count = 0;
        let version = self.config.version.clone();

        let Some(version) = version else {
            return Ok(count);
        };

        if self.tool.is_setup(&version).await? {
            debug!(target: self.tool.get_log_target(), "pnpm has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto::is_offline() {
            debug!(
                target: self.tool.get_log_target(),
                "No internet connection and pnpm has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            return Ok(count);
        }

        if let Some(last) = last_versions.get("pnpm") {
            if last == &version && self.tool.get_install_dir()?.exists() {
                return Ok(count);
            }
        }

        print_checkpoint(format!("installing pnpm v{version}"), Checkpoint::Setup);

        if self.tool.setup(&version).await? {
            last_versions.insert("pnpm".into(), version);
            count += 1;
        }

        Ok(count)
    }

    async fn teardown(&mut self) -> Result<(), ToolError> {
        self.tool.teardown().await?;

        Ok(())
    }
}

#[async_trait]
impl DependencyManager<NodeTool> for PnpmTool {
    fn create_command(&self, node: &NodeTool) -> Result<Command, ToolError> {
        let mut cmd = if self.global {
            Command::new("pnpm")
        } else if let Some(shim) = self.get_shim_path() {
            Command::new(shim)
        } else {
            let mut cmd = Command::new(node.get_bin_path()?);
            cmd.arg(self.get_bin_path()?);
            cmd
        };

        if !self.global {
            cmd.env("PATH", get_path_env_var(&self.tool.get_install_dir()?));
        }

        cmd.env("PROTO_NODE_BIN", node.get_bin_path()?);

        Ok(cmd)
    }

    async fn dedupe_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolError> {
        if working_dir.join(self.get_lock_filename()).exists() {
            let version = match self.config.version.as_ref() {
                Some(v) => v,
                None => "0.0.0",
            };

            // https://github.com/pnpm/pnpm/releases/tag/v7.26.0
            if semver::satisfies_range(version, ">=7.26.0") {
                self.create_command(node)?
                    .arg("dedupe")
                    .cwd(working_dir)
                    .log_running_command(log)
                    .exec_capture_output()
                    .await?;
            } else {
                node.exec_package("pnpm-deduplicate", &["pnpm-deduplicate"], working_dir)
                    .await?;
            }
        }

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(PNPM.lockfile)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(PNPM.manifest)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> Result<LockfileDependencyVersions, ToolError> {
        let Some(lockfile_path) = fs::find_upwards(PNPM.lockfile, project_root) else {
            return Ok(FxHashMap::default());
        };

        Ok(pnpm::load_lockfile_dependencies(lockfile_path)?)
    }

    async fn install_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolError> {
        let mut args = vec!["install"];

        if is_ci() {
            let lockfile = working_dir.join(self.get_lock_filename());

            // Will fail with "Headless installation requires a pnpm-lock.yaml file"
            if lockfile.exists() {
                args.push("--frozen-lockfile");
            }
        }

        let mut cmd = self.create_command(node)?;

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
    ) -> Result<(), ToolError> {
        let mut cmd = self.create_command(node)?;
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
            cmd.arg(format!("{package}..."));
        }

        cmd.exec_stream_output().await?;

        Ok(())
    }
}
