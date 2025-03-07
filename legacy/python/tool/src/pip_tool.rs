use crate::python_tool::{PythonTool, get_python_tool_paths};
use moon_config::PipConfig;
use moon_console::Console;
use moon_process::Command;
use moon_python_lang::{LockfileDependencyVersions, pip};
use moon_tool::{
    DependencyManager, Tool, async_trait, get_proto_env_vars, get_proto_version_env,
    prepend_path_env_var,
};
use moon_utils::get_workspace_root;
use proto_core::{ProtoEnvironment, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::instrument;

pub fn find_requirements_txt(starting_dir: &Path, workspace_root: &Path) -> Option<PathBuf> {
    fs::find_upwards_until("requirements.txt", starting_dir, workspace_root)
}

pub struct PipTool {
    pub config: PipConfig,

    console: Arc<Console>,

    global: bool,

    #[allow(dead_code)]
    proto_env: Arc<ProtoEnvironment>,
}

impl PipTool {
    pub async fn new(
        proto_env: Arc<ProtoEnvironment>,
        console: Arc<Console>,
        config: &PipConfig,
        global: bool,
    ) -> miette::Result<PipTool> {
        Ok(PipTool {
            global,
            config: config.to_owned(),
            proto_env,
            console,
        })
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
impl Tool for PipTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    #[instrument(skip_all)]
    async fn setup(
        &mut self,
        _last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        Ok(0)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DependencyManager<PythonTool> for PipTool {
    fn create_command(&self, python: &PythonTool) -> miette::Result<Command> {
        let mut cmd = Command::new("python");
        cmd.with_console(self.console.clone());
        cmd.envs(get_proto_env_vars());
        cmd.args(["-m", "pip"]);

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
        String::from("requirements.txt")
    }

    fn get_manifest_filename(&self) -> String {
        String::from("requirements.txt")
    }

    #[instrument(skip_all)]
    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> miette::Result<LockfileDependencyVersions> {
        let Some(reqs_path) = find_requirements_txt(project_root, &get_workspace_root()) else {
            return Ok(FxHashMap::default());
        };

        Ok(pip::load_lockfile_dependencies(reqs_path)?)
    }

    #[instrument(skip_all)]
    async fn install_dependencies(
        &self,
        python: &PythonTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let mut args: Vec<&str> = vec![];
        let reqs_path = find_requirements_txt(working_dir, &get_workspace_root());

        if let Some(reqs_path) = &reqs_path {
            args.extend(["-r", reqs_path.to_str().unwrap_or_default()]);
        }

        args.extend(
            self.config
                .install_args
                .iter()
                .map(|arg| arg.as_str())
                .collect::<Vec<_>>(),
        );

        if args.is_empty() {
            return Ok(());
        }

        let mut cmd = self.create_command(python)?;

        self.inject_command_paths(&mut cmd, python, working_dir);

        cmd.arg("install")
            .args(args)
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
        _package_names: &[String],
        _production_only: bool,
    ) -> miette::Result<()> {
        Ok(())
    }
}
