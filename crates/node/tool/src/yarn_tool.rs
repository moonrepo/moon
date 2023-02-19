use crate::node_tool::NodeTool;
use moon_config::YarnConfig;
use moon_logger::{color, debug};
use moon_node_lang::{yarn, LockfileDependencyVersions, YARN};
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{get_path_env_var, DependencyManager, Tool, ToolError};
use moon_utils::process::Command;
use moon_utils::{fs, get_workspace_root, is_ci};
use proto::{
    async_trait,
    node::{NodeDependencyManager, NodeDependencyManagerType},
    Describable, Executable, Installable, Proto, Shimable, Tool as ProtoTool,
};
use rustc_hash::FxHashMap;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct YarnTool {
    pub config: YarnConfig,

    pub global: bool,

    pub tool: NodeDependencyManager,
}

impl YarnTool {
    pub fn new(proto: &Proto, config: &Option<YarnConfig>) -> Result<YarnTool, ToolError> {
        let config = config.to_owned().unwrap_or_default();

        Ok(YarnTool {
            global: config.version.is_none(),
            config,
            tool: NodeDependencyManager::new(proto, NodeDependencyManagerType::Yarn),
        })
    }

    pub fn is_v1(&self) -> bool {
        self.config
            .version
            .as_ref()
            .map(|v| v.starts_with('1'))
            .unwrap_or(false)
    }

    pub async fn set_version(&mut self, node: &NodeTool) -> Result<(), ToolError> {
        if self.is_v1() {
            return Ok(());
        }

        let Some(version) = &self.config.version else {
            return Ok(());
        };

        let yarn_bin = get_workspace_root()
            .join(".yarn/releases")
            .join(format!("yarn-{version}.cjs"));

        if !yarn_bin.exists() {
            debug!(
                target: self.tool.get_log_target(),
                "Updating yarn version with {}",
                color::shell(format!("yarn set version {version}"))
            );

            self.create_command(node)?
                .args(["set", "version", version])
                .exec_capture_output()
                .await?;

            if let Some(plugins) = &self.config.plugins {
                for plugin in plugins {
                    self.create_command(node)?
                        .args(["plugin", "import", plugin])
                        .exec_capture_output()
                        .await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for YarnTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<PathBuf, ToolError> {
        Ok(if self.global {
            "yarn".into()
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
            debug!(target: self.tool.get_log_target(), "yarn has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto::is_offline() {
            debug!(
                target: self.tool.get_log_target(),
                "No internet connection and yarn has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            return Ok(count);
        }

        if let Some(last) = last_versions.get("yarn") {
            if last == &version && self.tool.get_install_dir()?.exists() {
                return Ok(count);
            }
        }

        print_checkpoint(format!("installing yarn v{version}"), Checkpoint::Setup);

        if self.tool.setup(&version).await? {
            last_versions.insert("yarn".into(), version);
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
impl DependencyManager<NodeTool> for YarnTool {
    fn create_command(&self, node: &NodeTool) -> Result<Command, ToolError> {
        let mut cmd = if self.global {
            Command::new("yarn")
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
        // Yarn v1 doesnt dedupe natively, so use:
        // npx yarn-deduplicate yarn.lock
        if self.is_v1() {
            // Will error if the lockfile does not exist!
            if working_dir.join(self.get_lock_filename()).exists() {
                node.exec_package(
                    "yarn-deduplicate",
                    &["yarn-deduplicate", YARN.lockfile],
                    working_dir,
                )
                .await?;
            }
        } else {
            self.create_command(node)?
                .arg("dedupe")
                .cwd(working_dir)
                .log_running_command(log)
                .exec_capture_output()
                .await?;
        }

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(YARN.lockfile)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(YARN.manifest)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> Result<LockfileDependencyVersions, ToolError> {
        let Some(lockfile_path) = fs::find_upwards(YARN.lockfile, project_root) else {
            return Ok(FxHashMap::default());
        };

        Ok(yarn::load_lockfile_dependencies(lockfile_path)?)
    }

    async fn install_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolError> {
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

        if self.is_v1() {
            cmd.arg("install");
        } else {
            cmd.args(["workspaces", "focus"]);
            cmd.args(packages);

            let workspace_plugin =
                get_workspace_root().join(".yarn/plugins/@yarnpkg/plugin-workspace-tools.cjs");

            if !workspace_plugin.exists() {
                return Err(ToolError::RequiresPlugin(
                    "yarn plugin import workspace-tools".into(),
                ));
            }
        };

        if production_only {
            cmd.arg("--production");
        }

        cmd.exec_stream_output().await?;

        Ok(())
    }
}
