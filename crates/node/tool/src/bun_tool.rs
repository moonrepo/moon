use crate::node_tool::NodeTool;
use moon_config::BunpmConfig;
use moon_logger::debug;
use moon_node_lang::{bun, LockfileDependencyVersions, BUN};
use moon_process::{output_to_string, Command};
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{
    async_trait, load_tool_plugin, prepend_path_env_var, use_global_tool_on_path,
    DependencyManager, Tool,
};
use moon_utils::get_workspace_root;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};

pub struct BunTool {
    pub config: BunpmConfig,

    pub global: bool,

    pub tool: ProtoTool,
}

impl BunTool {
    pub async fn new(
        proto: &ProtoEnvironment,
        config: &Option<BunpmConfig>,
    ) -> miette::Result<BunTool> {
        let config = config.to_owned().unwrap_or_default();

        Ok(BunTool {
            global: use_global_tool_on_path() || config.version.is_none(),
            tool: load_tool_plugin(&Id::raw("bun"), proto, config.plugin.as_ref().unwrap()).await?,
            config,
        })
    }

    fn internal_create_command(&self) -> miette::Result<Command> {
        Ok(if self.global {
            Command::new("bun")
        } else if let Some(shim) = self.get_shim_path() {
            Command::new(shim)
        } else {
            Command::new(self.get_bin_path()?)
        })
    }
}

#[async_trait]
impl Tool for BunTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    fn get_bin_path(&self) -> miette::Result<PathBuf> {
        Ok(if self.global {
            "bun".into()
        } else {
            self.tool.get_exe_path()?.to_path_buf()
        })
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let mut count = 0;
        let version = self.config.version.as_ref();

        let Some(version) = version else {
            return Ok(count);
        };

        if self.global {
            debug!("Using global binary in PATH");

            return Ok(count);
        }

        if self.tool.is_setup(version).await? {
            self.tool.locate_globals_dir().await?;

            debug!("bun has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto_core::is_offline() {
            debug!(
                "No internet connection and bun has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            return Ok(count);
        }

        if let Some(last) = last_versions.get("bun") {
            if last == version && self.tool.get_tool_dir().exists() {
                return Ok(count);
            }
        }

        print_checkpoint(format!("installing bun {version}"), Checkpoint::Setup);

        if self.tool.setup(version, false).await? {
            last_versions.insert("bun".into(), version.to_owned());
            count += 1;
        }

        self.tool.locate_globals_dir().await?;

        Ok(count)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;

        Ok(())
    }
}

#[async_trait]
impl DependencyManager<NodeTool> for BunTool {
    fn create_command(&self, _node: &NodeTool) -> miette::Result<Command> {
        let mut cmd = self.internal_create_command()?;

        if !self.global {
            cmd.env(
                "PATH",
                prepend_path_env_var([self.tool.get_exe_path()?.parent().unwrap()]),
            );
        }

        Ok(cmd)
    }

    async fn dedupe_dependencies(
        &self,
        _node: &NodeTool,
        _working_dir: &Path,
        _log: bool,
    ) -> miette::Result<()> {
        // Not supported!

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(BUN.lockfile)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(BUN.manifest)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> miette::Result<LockfileDependencyVersions> {
        let Some(lockfile_path) =
            fs::find_upwards_until(BUN.lockfile, project_root, get_workspace_root())
        else {
            return Ok(FxHashMap::default());
        };

        // Bun lockfiles are binary, so we need to convert them to text first
        // using Bun itself!
        let mut cmd = self.internal_create_command()?;
        cmd.arg(BUN.lockfile);
        cmd.cwd(lockfile_path.parent().unwrap());

        let output = cmd.create_async().exec_capture_output().await?;

        Ok(bun::load_lockfile_dependencies(output_to_string(
            &output.stdout,
        ))?)
    }

    async fn install_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(node)?;

        cmd.args(["install"])
            .cwd(working_dir)
            .set_print_command(log);

        let mut cmd = cmd.create_async();

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
        _package_names: &[String], // Not supporetd
        _production_only: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(node)?;
        cmd.args(["install"]);

        // NOTE: This seems to *not* install any dependencies
        // if production_only {
        //     cmd.arg("--production");
        // }

        cmd.create_async().exec_stream_output().await?;

        Ok(())
    }
}
