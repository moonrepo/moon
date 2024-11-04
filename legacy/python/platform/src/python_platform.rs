use crate::{actions, find_requirements_txt, toolchain_hash::PythonToolchainHash};
use moon_action::Operation;
use moon_action_context::ActionContext;
use moon_common::{color, Id};
use moon_config::{
    HasherConfig, PlatformType, ProjectConfig, ProjectsAliasesList, ProjectsSourcesList,
    PythonConfig, UnresolvedVersionSpec,
};
use moon_console::Console;
use moon_hash::ContentHasher;
use moon_logger::debug;
use moon_platform::{Platform, Runtime, RuntimeReq};
use moon_process::Command;
use moon_project::Project;
use moon_python_lang::pip_requirements::load_lockfile_dependencies;
use moon_python_tool::{get_python_tool_paths, PythonTool};
use moon_task::Task;
use moon_tool::{get_proto_version_env, prepend_path_env_var, Tool, ToolManager};
use moon_utils::{async_trait, get_workspace_root};
use proto_core::ProtoEnvironment;
use rustc_hash::FxHashMap;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::instrument;

const LOG_TARGET: &str = "moon:python-platform";

pub struct PythonPlatform {
    pub config: PythonConfig,

    console: Arc<Console>,

    proto_env: Arc<ProtoEnvironment>,

    toolchain: ToolManager<PythonTool>,

    #[allow(dead_code)]
    pub workspace_root: PathBuf,
}

impl PythonPlatform {
    pub fn new(
        config: &PythonConfig,
        workspace_root: &Path,
        proto_env: Arc<ProtoEnvironment>,
        console: Arc<Console>,
    ) -> Self {
        PythonPlatform {
            config: config.to_owned(),
            proto_env,
            toolchain: ToolManager::new(Runtime::new(PlatformType::Python, RuntimeReq::Global)),
            workspace_root: workspace_root.to_path_buf(),
            console,
        }
    }
}

#[async_trait]
impl Platform for PythonPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::Python
    }

    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Runtime {
        if let Some(config) = &project_config {
            if let Some(python_config) = &config.toolchain.python {
                if let Some(version) = &python_config.version {
                    return Runtime::new_override(
                        PlatformType::Python,
                        RuntimeReq::Toolchain(version.to_owned()),
                    );
                }
            }
        }

        if let Some(version) = &self.config.version {
            return Runtime::new(
                PlatformType::Python,
                RuntimeReq::Toolchain(version.to_owned()),
            );
        }

        Runtime::new(PlatformType::Python, RuntimeReq::Global)
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::Python) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime.platform, PlatformType::Python);
        }

        false
    }

    // PROJECT GRAPH

    fn is_project_in_dependency_workspace(&self, _project_source: &str) -> miette::Result<bool> {
        // Single version policy / only a root requirements.txt
        Ok(true)
    }

    #[instrument(skip_all)]
    fn load_project_graph_aliases(
        &mut self,
        _projects_list: &ProjectsSourcesList,
        _aliases_list: &mut ProjectsAliasesList,
    ) -> miette::Result<()> {
        // Not supported
        Ok(())
    }

    // TOOLCHAIN

    fn is_toolchain_enabled(&self) -> miette::Result<bool> {
        Ok(self.config.version.is_some())
    }

    fn get_tool(&self) -> miette::Result<Box<&dyn Tool>> {
        let tool = self.toolchain.get()?;

        Ok(Box::new(tool))
    }

    fn get_tool_for_version(&self, req: RuntimeReq) -> miette::Result<Box<&dyn Tool>> {
        let tool = self.toolchain.get_for_version(&req)?;

        Ok(Box::new(tool))
    }

    fn get_dependency_configs(&self) -> miette::Result<Option<(String, String)>> {
        Ok(Some((
            "requirements.txt".to_owned(),
            "requirements.txt".to_owned(),
        )))
    }

    async fn setup_toolchain(&mut self) -> miette::Result<()> {
        let req = match &self.config.version {
            Some(v) => RuntimeReq::Toolchain(v.to_owned()),
            None => RuntimeReq::Global,
        };

        let mut last_versions = FxHashMap::default();

        if !self.toolchain.has(&req) {
            self.toolchain.register(
                &req,
                PythonTool::new(
                    Arc::clone(&self.proto_env),
                    Arc::clone(&self.console),
                    &self.config,
                    &req,
                )
                .await?,
            );
        }

        self.toolchain.setup(&req, &mut last_versions).await?;

        Ok(())
    }

    async fn teardown_toolchain(&mut self) -> miette::Result<()> {
        self.toolchain.teardown_all().await?;

        Ok(())
    }

    // ACTIONS

    #[instrument(skip_all)]
    async fn setup_tool(
        &mut self,
        _context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let req = &runtime.requirement;

        if !self.toolchain.has(req) {
            self.toolchain.register(
                req,
                PythonTool::new(
                    Arc::clone(&self.proto_env),
                    Arc::clone(&self.console),
                    &self.config,
                    req,
                )
                .await?,
            );
        }

        let installed = self.toolchain.setup(req, last_versions).await?;

        actions::setup_tool(self.toolchain.get_for_version(req)?, &self.workspace_root).await?;
        actions::install_deps(
            self.toolchain.get_for_version(req)?,
            &self.workspace_root,
            &self.console,
        )
        .await?;

        Ok(installed)
    }

    #[instrument(skip_all)]
    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        _working_dir: &Path,
    ) -> miette::Result<Vec<Operation>> {
        actions::install_deps(
            self.toolchain.get_for_version(&runtime.requirement)?,
            &get_workspace_root(),
            &self.console,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn sync_project(
        &self,
        _context: &ActionContext,
        _project: &Project,
        _dependencies: &FxHashMap<Id, Arc<Project>>,
    ) -> miette::Result<bool> {
        let mutated_files = false;
        //TODO: Here we can modifiy something

        Ok(mutated_files)
    }

    #[instrument(skip_all)]
    async fn hash_manifest_deps(
        &self,
        manifest_path: &Path,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        if let Some(python_version) = &self.config.version {
            let deps =
                BTreeMap::from_iter(load_lockfile_dependencies(manifest_path.to_path_buf())?);
            debug!(
                target: LOG_TARGET,
                "HASH MANIFEST {}",
                color::path(manifest_path)
            );
            hasher.hash_content(PythonToolchainHash {
                version: python_version.clone(),
                dependencies: deps,
            })?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn hash_run_target(
        &self,
        project: &Project,
        _runtime: &Runtime,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        if let Some(python_version) = &self.config.version {
            let mut deps = BTreeMap::new();
            if let Some(pip_requirements) =
                find_requirements_txt(&project.root, &self.workspace_root)
            {
                deps = BTreeMap::from_iter(load_lockfile_dependencies(pip_requirements)?);
            }
            debug!(
                target: LOG_TARGET,
                "HASH RUN TARGET {}",
                color::path(&project.root)
            );
            hasher.hash_content(PythonToolchainHash {
                version: python_version.clone(),
                dependencies: deps,
            })?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn create_run_target_command(
        &self,
        _context: &ActionContext,
        _project: &Project,
        task: &Task,
        runtime: &Runtime,
        _working_dir: &Path,
    ) -> miette::Result<Command> {
        let mut command = Command::new(&task.command);
        command.with_console(self.console.clone());
        command.args(&task.args);
        command.envs(&task.env);

        if let Ok(python) = self.toolchain.get_for_version(&runtime.requirement) {
            if let Some(version) = get_proto_version_env(&python.tool) {
                command.env("PROTO_PYTHON_VERSION", version);
                command.env("PATH", prepend_path_env_var(get_python_tool_paths(python)));
            }
        }

        Ok(command)
    }
}
