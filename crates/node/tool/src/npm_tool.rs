use crate::node_tool::NodeTool;
use moon_config::NpmConfig;
use moon_lang::LockfileDependencyVersions;
use moon_logger::debug;
use moon_node_lang::{npm, NPM};
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{get_path_env_var, DependencyManager, Tool, ToolError};
use moon_utils::process::Command;
use moon_utils::{fs, is_ci};
use proto_core::{async_trait, Describable, Executable, Proto, Resolvable, Tool as ProtoTool};
use proto_node::NodeDependencyManager;
use rustc_hash::FxHashMap;
use std::env;
use std::path::Path;

#[derive(Debug)]
pub struct NpmTool {
    pub config: NpmConfig,

    tool: NodeDependencyManager,
}

impl NpmTool {
    pub fn new(proto: &Proto, config: &NpmConfig) -> Result<NpmTool, ToolError> {
        Ok(NpmTool {
            config: config.to_owned(),
            tool: NodeDependencyManager::new(
                proto,
                proto_node::NodeDependencyManagerType::Npm,
                Some(&config.version),
            ),
        })
    }
}

#[async_trait]
impl Tool for NpmTool {
    fn get_bin_path(&self) -> Result<&Path, ToolError> {
        Ok(self.tool.get_bin_path()?)
    }

    fn get_version(&self) -> &str {
        self.tool.get_resolved_version()
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        let mut count = 0;

        if self.tool.is_setup(&self.config.version).await? {
            debug!(target: self.tool.get_log_target(), "npm has already been setup");

            return Ok(count);
        }

        if let Some(last) = last_versions.get("npm") {
            if last == &self.config.version {
                return Ok(count);
            }
        }

        print_checkpoint(
            format!("installing node v{}", self.config.version),
            Checkpoint::Setup,
        );

        if self.tool.setup(&self.config.version).await? {
            last_versions.insert("npm".into(), self.config.version.clone());
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
impl DependencyManager<NodeTool> for NpmTool {
    fn create_command(&self, node: &NodeTool) -> Result<Command, ToolError> {
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
    ) -> Result<(), ToolError> {
        self.create_command(node)?
            .args(["dedupe"])
            .cwd(working_dir)
            .log_running_command(log)
            .exec_capture_output()
            .await?;

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(NPM.lockfile)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(NPM.manifest)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> Result<LockfileDependencyVersions, ToolError> {
        let Some(lockfile_path) = fs::find_upwards(NPM.lockfile, project_root) else {
            return Ok(FxHashMap::default());
        };

        Ok(npm::load_lockfile_dependencies(lockfile_path)?)
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

            // npm will error if using `ci` and a lockfile does not exist!
            if lockfile.exists() {
                args.clear();
                args.push("ci");
            }
        } else {
            args.push("--no-audit");
        }

        args.push("--no-fund");

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
        package_names: &[String],
        production_only: bool,
    ) -> Result<(), ToolError> {
        let mut cmd = self.create_command(node)?;
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
