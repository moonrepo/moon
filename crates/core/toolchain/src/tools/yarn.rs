use crate::get_path_env_var;
use crate::tools::node::NodeTool;
use crate::{errors::ToolchainError, DependencyManager, RuntimeTool};
use moon_config::YarnConfig;
use moon_lang::LockfileDependencyVersions;
use moon_logger::{color, debug};
use moon_node_lang::{yarn, YARN};
use moon_utils::process::Command;
use moon_utils::{fs, get_workspace_root, is_ci};
use probe_core::{async_trait, Describable, Executable, Probe, Resolvable, Tool};
use probe_node::NodeDependencyManager;
use rustc_hash::FxHashMap;
use std::env;
use std::path::Path;

#[derive(Debug)]
pub struct YarnTool {
    pub config: YarnConfig,

    tool: NodeDependencyManager,
}

impl YarnTool {
    pub fn new(probe: &Probe, config: &Option<YarnConfig>) -> Result<YarnTool, ToolchainError> {
        Ok(YarnTool {
            config: config.to_owned().unwrap_or_default(),
            tool: NodeDependencyManager::new(
                probe,
                probe_node::NodeDependencyManagerType::Yarn,
                match &config {
                    Some(cfg) => Some(&cfg.version),
                    None => None,
                },
            ),
        })
    }

    pub fn is_v1(&self) -> bool {
        self.config.version.starts_with('1')
    }

    pub async fn set_version(&mut self, node: &NodeTool) -> Result<(), ToolchainError> {
        if self.is_v1() {
            return Ok(());
        }

        let yarn_bin = get_workspace_root()
            .join(".yarn/releases")
            .join(format!("yarn-{}.cjs", self.config.version));

        if !yarn_bin.exists() {
            debug!(
                target: self.tool.get_log_target(),
                "Updating yarn version with {}",
                color::shell(format!("yarn set version {}", self.config.version))
            );

            self.create_command(node)?
                .args(["set", "version", &self.config.version])
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
impl RuntimeTool for YarnTool {
    fn get_bin_path(&self) -> Result<&Path, ToolchainError> {
        Ok(self.tool.get_bin_path()?)
    }

    fn get_version(&self) -> &str {
        self.tool.get_resolved_version()
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolchainError> {
        let mut count = 0;

        if self.tool.is_setup().await? {
            return Ok(count);
        } else if let Some(last) = last_versions.get("yarn") {
            if last == &self.config.version {
                return Ok(count);
            }
        }

        if self.tool.setup(&self.config.version).await? {
            last_versions.insert("yarn".into(), self.config.version.clone());
            count += 1;
        }

        Ok(count)
    }

    async fn teardown(&mut self) -> Result<(), ToolchainError> {
        self.tool.teardown().await?;

        Ok(())
    }
}

#[async_trait]
impl DependencyManager<NodeTool> for YarnTool {
    fn create_command(&self, node: &NodeTool) -> Result<Command, ToolchainError> {
        let bin_path = self.get_bin_path()?;

        let mut cmd = Command::new(node.get_bin_path()?);
        cmd.env("PATH", get_path_env_var(bin_path.parent().unwrap()));
        cmd.arg(bin_path);

        Ok(cmd)
    }

    async fn dedupe_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> Result<(), ToolchainError> {
        // Yarn v1 doesnt dedupe natively, so use:
        // npx yarn-deduplicate yarn.lock
        if self.is_v1() {
            // Will error if the lockfile does not exist!
            if working_dir.join(self.get_lock_filename()).exists() {
                node.exec_package(
                    "yarn-deduplicate",
                    &["yarn-deduplicate", YARN.lock_filename],
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
    ) -> Result<(), ToolchainError> {
        let mut cmd = self.create_command(node)?;

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
