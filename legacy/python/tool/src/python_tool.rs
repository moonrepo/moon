use moon_config::PythonConfig;
use moon_console::{Checkpoint, Console};
use moon_logger::{debug, map_list};
use moon_process::Command;
use moon_tool::{
    async_trait, get_proto_env_vars, get_proto_paths, load_tool_plugin, prepend_path_env_var,
    use_global_tool_on_path, Tool,
};
use moon_toolchain::RuntimeReq;
use moon_utils::get_workspace_root;
use proto_core::flow::install::InstallOptions;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use starbase_styles::color;
use std::path::PathBuf;
use std::sync::Arc;
use std::{ffi::OsStr, path::Path};
use tracing::instrument;

const LOG_TARGET: &str = "moon:python-tool";

pub fn get_python_tool_paths(python_tool: &PythonTool) -> Vec<PathBuf> {
    // let mut paths = get_proto_paths(proto_env);
    // let mut paths:Vec<PathBuf> = [];

    // let mut python_command = "python";

    let venv_python = &get_workspace_root().join(python_tool.config.venv_name.clone());

    let paths;

    if venv_python.exists() {
        paths = vec![
            venv_python.join("Scripts").clone(),
            venv_python.join("bin").clone(),
        ];
    } else {
        // paths = python_tool.tool.get_globals_dirs()
        //     .iter()
        //     .cloned()
        //     .collect::<Vec<PathBuf>>();
        paths = get_proto_paths(&python_tool.proto_env);
    }

    debug!(
        target: LOG_TARGET,
        "Proto Env {} ",
        map_list(&paths, |c| color::label(c.display().to_string())),
    );
    paths
}

pub struct PythonTool {
    pub config: PythonConfig,

    pub global: bool,

    pub tool: ProtoTool,

    console: Arc<Console>,

    proto_env: Arc<ProtoEnvironment>,
}

impl PythonTool {
    pub async fn new(
        proto_env: Arc<ProtoEnvironment>,
        console: Arc<Console>,
        config: &PythonConfig,
        req: &RuntimeReq,
    ) -> miette::Result<PythonTool> {
        let mut python = PythonTool {
            config: config.to_owned(),
            global: false,
            tool: load_tool_plugin(
                &Id::raw("python"),
                &proto_env,
                config.plugin.as_ref().unwrap(),
            )
            .await?,
            proto_env,
            console,
        };

        if use_global_tool_on_path("python") || req.is_global() {
            python.global = true;
            python.config.version = None;
        } else {
            python.config.version = req.to_spec();
        };

        Ok(python)
    }

    #[instrument(skip_all)]
    pub async fn exec_python<I, S>(&self, args: I, working_dir: &Path) -> miette::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("python")
            .args(args)
            .envs(get_proto_env_vars())
            .env("PATH", prepend_path_env_var(get_python_tool_paths(&self)))
            .cwd(working_dir)
            .with_console(self.console.clone())
            .create_async()
            .exec_stream_output()
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Tool for PythonTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    #[instrument(skip_all)]
    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let mut installed = 0;

        let Some(version) = &self.config.version else {
            return Ok(installed);
        };

        if self.global {
            debug!("Using global binary in PATH");
        } else if self.tool.is_setup(version).await? {
            debug!("Python has already been setup");

            // When offline and the tool doesn't exist, fallback to the global binary
        } else if proto_core::is_offline() {
            debug!(
                "No internet connection and Python has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            // Otherwise try and install the tool
        } else {
            let setup = match last_versions.get("python") {
                Some(last) => version != last,
                None => true,
            };

            if setup || !self.tool.get_product_dir().exists() {
                self.console
                    .out
                    .print_checkpoint(Checkpoint::Setup, format!("installing python {version}"))?;

                if self.tool.setup(version, InstallOptions::default()).await? {
                    last_versions.insert("python".into(), version.to_owned());
                    installed += 1;
                }
            }
        }
        self.tool.locate_globals_dirs().await?;
        Ok(installed)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;
        Ok(())
    }
}
