use crate::python_tool::{PythonTool, get_python_tool_paths};
use moon_config::UvConfig;
use moon_console::{Checkpoint, Console};
use moon_logger::debug;
use moon_process::Command;
use moon_python_lang::{LockfileDependencyVersions, uv};
use moon_tool::{
    DependencyManager, Tool, async_trait, get_proto_env_vars, get_proto_version_env,
    load_tool_plugin, prepend_path_env_var, use_global_tool_on_path,
};
use moon_utils::get_workspace_root;
use proto_core::flow::install::InstallOptions;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::env;
use std::path::Path;
use std::sync::Arc;
use tracing::instrument;

pub struct UvTool {
    pub config: UvConfig,

    pub global: bool,

    pub tool: ProtoTool,

    console: Arc<Console>,

    #[allow(dead_code)]
    proto_env: Arc<ProtoEnvironment>,
}

impl UvTool {
    pub async fn new(
        proto_env: Arc<ProtoEnvironment>,
        console: Arc<Console>,
        config: &Option<UvConfig>,
    ) -> miette::Result<UvTool> {
        let config = config.to_owned().unwrap_or_default();

        Ok(UvTool {
            global: use_global_tool_on_path("uv") || config.version.is_none(),
            tool: load_tool_plugin(&Id::raw("uv"), &proto_env, config.plugin.as_ref().unwrap())
                .await?,
            config,
            proto_env,
            console,
        })
    }

    pub fn create_command_with_paths(
        &self,
        python: &PythonTool,
        working_dir: &Path,
    ) -> miette::Result<Command> {
        let mut cmd = self.create_command(python)?;
        self.inject_command_paths(&mut cmd, python, working_dir);
        Ok(cmd)
    }

    fn inject_command_paths(&self, cmd: &mut Command, python: &PythonTool, working_dir: &Path) {
        if !self.global {
            cmd.env(
                "PATH",
                prepend_path_env_var(get_python_tool_paths(
                    python,
                    working_dir,
                    &get_workspace_root(),
                )),
            );
        }
    }
}

#[async_trait]
impl Tool for UvTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    #[instrument(skip_all)]
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
            self.tool.locate_globals_dirs().await?;

            debug!("uv has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto_core::is_offline() {
            debug!(
                "No internet connection and uv has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            return Ok(count);
        }

        if let Some(last) = last_versions.get("uv") {
            if last == version && self.tool.get_product_dir().exists() {
                return Ok(count);
            }
        }

        self.console
            .print_checkpoint(Checkpoint::Setup, format!("installing uv {version}"))?;

        if self.tool.setup(version, InstallOptions::default()).await? {
            last_versions.insert("uv".into(), version.to_owned());
            count += 1;
        }

        self.tool.locate_globals_dirs().await?;

        Ok(count)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;

        Ok(())
    }
}

#[async_trait]
impl DependencyManager<PythonTool> for UvTool {
    fn create_command(&self, python: &PythonTool) -> miette::Result<Command> {
        let mut cmd = Command::new("uv");
        cmd.with_console(self.console.clone());
        cmd.envs(get_proto_env_vars());

        if let Some(version) = get_proto_version_env(&self.tool) {
            cmd.env("PROTO_UV_VERSION", version);
        }

        if let Some(version) = get_proto_version_env(&python.tool) {
            cmd.env("PROTO_PYTHON_VERSION", version);
        }

        Ok(cmd)
    }

    #[instrument(skip_all)]
    async fn dedupe_dependencies(
        &self,
        _python: &PythonTool,
        _working_dir: &Path,
        _log: bool,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from("uv.lock")
    }

    fn get_manifest_filename(&self) -> String {
        String::from("pyproject.toml")
    }

    #[instrument(skip_all)]
    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> miette::Result<LockfileDependencyVersions> {
        let Some(lockfile_path) =
            fs::find_upwards_until("uv.lock", project_root, get_workspace_root())
        else {
            return Ok(FxHashMap::default());
        };

        Ok(uv::load_lockfile_dependencies(lockfile_path)?)
    }

    #[instrument(skip_all)]
    async fn install_dependencies(
        &self,
        python: &PythonTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(python)?;

        self.inject_command_paths(&mut cmd, python, working_dir);

        cmd.args(["sync"])
            .args(&self.config.sync_args)
            .cwd(working_dir)
            .set_print_command(log);

        if env::var("MOON_TEST_HIDE_INSTALL_OUTPUT").is_ok() {
            cmd.exec_capture_output().await?;
        } else {
            cmd.exec_stream_output().await?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn install_focused_dependencies(
        &self,
        _python: &PythonTool,
        _packages: &[String],
        _production_only: bool,
    ) -> miette::Result<()> {
        Ok(())
    }
}
