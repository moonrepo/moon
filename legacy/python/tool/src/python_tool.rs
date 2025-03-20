use crate::pip_tool::PipTool;
use crate::uv_tool::UvTool;
use moon_config::{PythonConfig, PythonPackageManager};
use moon_console::{Checkpoint, Console};
use moon_logger::debug;
use moon_process::Command;
use moon_tool::{
    DependencyManager, Tool, ToolError, async_trait, get_proto_env_vars, get_proto_paths,
    get_proto_version_env, load_tool_plugin, prepend_path_env_var, use_global_tool_on_path,
};
use moon_toolchain::RuntimeReq;
use proto_core::flow::install::InstallOptions;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::{ffi::OsStr, path::Path};
use tracing::instrument;

pub fn get_python_tool_paths(
    python_tool: &PythonTool,
    working_dir: &Path,
    workspace_root: &Path,
) -> Vec<PathBuf> {
    let mut paths = vec![];

    if let Some(venv_root) =
        fs::find_upwards_until(&python_tool.config.venv_name, working_dir, workspace_root)
    {
        paths.push(venv_root.join("Scripts"));
        paths.push(venv_root.join("bin"));
    }

    paths.extend(get_proto_paths(&python_tool.proto_env));
    paths
}

pub struct PythonTool {
    pub config: PythonConfig,

    pub global: bool,

    pub tool: ProtoTool,

    console: Arc<Console>,

    proto_env: Arc<ProtoEnvironment>,

    pip: Option<PipTool>,

    uv: Option<UvTool>,
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
            proto_env: Arc::clone(&proto_env),
            console: Arc::clone(&console),
            pip: None,
            uv: None,
        };

        if use_global_tool_on_path("python") || req.is_global() {
            python.global = true;
            python.config.version = None;
        } else {
            python.config.version = req.to_spec();
        };

        match config.package_manager {
            PythonPackageManager::Pip => {
                python.pip = Some(
                    PipTool::new(
                        Arc::clone(&proto_env),
                        Arc::clone(&console),
                        &config.pip,
                        python.global,
                    )
                    .await?,
                );
            }
            PythonPackageManager::Uv => {
                python.uv = Some(
                    UvTool::new(Arc::clone(&proto_env), Arc::clone(&console), &config.uv).await?,
                );
            }
        };

        Ok(python)
    }

    #[instrument(skip_all)]
    pub async fn exec_python<I, S>(
        &self,
        args: I,
        working_dir: &Path,
        workspace_root: &Path,
        with_paths: bool,
    ) -> miette::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new("python");

        cmd.args(args)
            .envs(get_proto_env_vars())
            .cwd(working_dir)
            .with_console(self.console.clone());

        if with_paths {
            cmd.env(
                "PATH",
                prepend_path_env_var(get_python_tool_paths(self, working_dir, workspace_root)),
            );
        } else {
            cmd.env(
                "PATH",
                prepend_path_env_var(get_proto_paths(&self.proto_env)),
            );
        }

        if let Some(version) = get_proto_version_env(&self.tool) {
            cmd.env("PROTO_PYTHON_VERSION", version);
        }

        cmd.exec_stream_output().await?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn exec_venv(
        &self,
        venv_root: &Path,
        working_dir: &Path,
        workspace_root: &Path,
    ) -> miette::Result<()> {
        match self.config.package_manager {
            PythonPackageManager::Pip => {
                self.exec_python(
                    [
                        "-m",
                        "venv",
                        venv_root.to_str().unwrap_or_default(),
                        "--clear",
                    ],
                    working_dir,
                    workspace_root,
                    false,
                )
                .await?;
            }
            PythonPackageManager::Uv => {
                let uv = self.get_uv()?;

                uv.create_command(self)?
                    .args([
                        "venv",
                        venv_root.to_str().unwrap_or_default(),
                        "--no-python-downloads",
                    ])
                    .cwd(working_dir)
                    .exec_stream_output()
                    .await?;
            }
        };

        Ok(())
    }

    pub fn get_pip(&self) -> miette::Result<&PipTool> {
        match &self.pip {
            Some(pip) => Ok(pip),
            None => Err(ToolError::UnknownTool("pip".into()).into()),
        }
    }

    pub fn get_uv(&self) -> miette::Result<&UvTool> {
        match &self.uv {
            Some(uv) => Ok(uv),
            None => Err(ToolError::UnknownTool("uv".into()).into()),
        }
    }

    pub fn get_package_manager(&self) -> &(dyn DependencyManager<Self> + Send + Sync) {
        if self.uv.is_some() {
            return self.get_uv().unwrap();
        }

        if self.pip.is_some() {
            return self.get_pip().unwrap();
        }

        panic!("No package manager, how's this possible?");
    }

    pub fn find_venv_root(&self, starting_dir: &Path, workspace_root: &Path) -> Option<PathBuf> {
        let depman = self.get_package_manager();

        fs::find_upwards_root_until(depman.get_manifest_filename(), starting_dir, workspace_root)
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
                    .print_checkpoint(Checkpoint::Setup, format!("installing python {version}"))?;

                if self.tool.setup(version, InstallOptions::default()).await? {
                    last_versions.insert("python".into(), version.to_owned());
                    installed += 1;
                }
            }
        }

        self.tool.locate_globals_dirs().await?;

        if let Some(pip) = &mut self.pip {
            installed += pip.setup(last_versions).await?;
        }

        if let Some(uv) = &mut self.uv {
            installed += uv.setup(last_versions).await?;
        }

        Ok(installed)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;

        Ok(())
    }
}
