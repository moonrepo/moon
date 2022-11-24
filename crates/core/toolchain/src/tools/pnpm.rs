use crate::get_path_env_var;
use crate::tools::node::NodeTool;
use crate::{errors::ToolchainError, DependencyManager, RuntimeTool};
use moon_config::PnpmConfig;
use moon_lang::LockfileDependencyVersions;
use moon_node_lang::{pnpm, PNPM};
use moon_utils::process::Command;
use moon_utils::{fs, is_ci};
use probe_core::{async_trait, Executable, Probe, Resolvable, Tool};
use probe_node::NodeDependencyManager;
use rustc_hash::FxHashMap;
use std::env;
use std::path::Path;

#[derive(Debug)]
pub struct PnpmTool {
    pub config: PnpmConfig,

    tool: NodeDependencyManager,
}

impl PnpmTool {
    pub fn new(probe: &Probe, config: &Option<PnpmConfig>) -> Result<PnpmTool, ToolchainError> {
        Ok(PnpmTool {
            config: config.to_owned().unwrap_or_default(),
            tool: NodeDependencyManager::new(
                probe,
                probe_node::NodeDependencyManagerType::Pnpm,
                match &config {
                    Some(cfg) => Some(&cfg.version),
                    None => None,
                },
            ),
        })
    }
}

#[async_trait]
impl RuntimeTool for PnpmTool {
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

        if self.tool.is_setup()? {
            return Ok(count);
        } else if let Some(last) = last_versions.get("pnpm") {
            if last == &self.config.version {
                return Ok(count);
            }
        }

        if self.tool.setup(&self.config.version).await? {
            last_versions.insert("pnpm".into(), self.get_version().to_owned());
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
impl DependencyManager<NodeTool> for PnpmTool {
    fn create_command(&self, node: &NodeTool) -> Result<Command, ToolchainError> {
        let bin_path = self.get_bin_path()?;

        let mut cmd = Command::new(node.get_bin_path()?);
        cmd.env("PATH", get_path_env_var(bin_path.parent().unwrap()));
        cmd.arg(bin_path);

        Ok(cmd)
    }

    async fn dedupe_dependencies(
        &self,
        _node: &NodeTool,
        _working_dir: &Path,
        _log: bool,
    ) -> Result<(), ToolchainError> {
        // pnpm doesn't support deduping, but maybe prune is good here?
        // https://pnpm.io/cli/prune
        // self.create_command(node)
        //     .arg("prune")
        //     .cwd(working_dir)
        //     .log_running_command(log)
        //     .exec_capture_output()
        //     .await?;

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
