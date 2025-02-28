use crate::{actions, toolchain_hash::PythonToolchainHash};
use moon_action::Operation;
use moon_action_context::ActionContext;
use moon_common::{
    Id, color,
    path::{WorkspaceRelativePath, is_root_level_source},
};
use moon_config::{
    DependencyConfig, DependencyScope, DependencySource, HasherConfig, HasherOptimization,
    PlatformType, ProjectConfig, ProjectsAliasesList, ProjectsSourcesList, PythonConfig,
    PythonPackageManager, UnresolvedVersionSpec,
};
use moon_console::Console;
use moon_hash::{ContentHasher, DepsHash};
use moon_platform::{Platform, Runtime, RuntimeReq};
use moon_process::Command;
use moon_project::Project;
use moon_python_lang::{pip, uv};
use moon_python_tool::{PythonTool, get_python_tool_paths};
use moon_task::Task;
use moon_tool::{Tool, ToolManager, get_proto_version_env, prepend_path_env_var};
use moon_utils::async_trait;
use proto_core::ProtoEnvironment;
use rustc_hash::FxHashMap;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{debug, instrument};

pub struct PythonPlatform {
    pub config: PythonConfig,

    console: Arc<Console>,

    package_names: FxHashMap<String, Id>,

    proto_env: Arc<ProtoEnvironment>,

    toolchain: ToolManager<PythonTool>,

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
            toolchain: ToolManager::new(Runtime::new(Id::raw("python"), RuntimeReq::Global)),
            workspace_root: workspace_root.to_path_buf(),
            console,
            package_names: FxHashMap::default(),
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
                        Id::raw("python"),
                        RuntimeReq::Toolchain(version.to_owned()),
                    );
                }
            }
        }

        if let Some(version) = &self.config.version {
            return Runtime::new(Id::raw("python"), RuntimeReq::Toolchain(version.to_owned()));
        }

        Runtime::new(Id::raw("python"), RuntimeReq::Global)
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::Python) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return runtime.toolchain == "python";
        }

        false
    }

    // PROJECT GRAPH

    fn is_project_in_dependency_workspace(
        &self,
        _deps_root: &WorkspaceRelativePath,
        project_source: &str,
    ) -> miette::Result<bool> {
        // Single version policy / only a root requirements.txt
        if self.config.root_venv_only {
            return Ok(true);
        }

        if is_root_level_source(project_source) {
            return Ok(true);
        }

        Ok(false)
    }

    #[instrument(skip_all)]
    fn load_project_graph_aliases(
        &mut self,
        projects_list: &ProjectsSourcesList,
        aliases_list: &mut ProjectsAliasesList,
    ) -> miette::Result<()> {
        if self.config.package_manager == PythonPackageManager::Uv {
            debug!(
                "Loading names (aliases) from project {}'s",
                color::file("pyproject.toml")
            );

            for (project_id, project_source) in projects_list {
                if let Some(data) =
                    uv::PyProjectTomlCache::read(project_source.to_path(&self.workspace_root))?
                {
                    if let Some(project) = data.project {
                        let package_name = project.name;

                        self.package_names
                            .insert(package_name.clone(), project_id.to_owned());

                        if package_name == project_id.as_str() {
                            continue;
                        }

                        debug!(
                            "Inheriting alias {} for project {}",
                            color::label(&package_name),
                            color::id(project_id)
                        );

                        aliases_list.push((project_id.to_owned(), package_name.clone()));
                    }
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    fn load_project_implicit_dependencies(
        &self,
        project_id: &str,
        project_source: &str,
    ) -> miette::Result<Vec<DependencyConfig>> {
        let mut implicit_deps = vec![];

        if self.config.package_manager == PythonPackageManager::Uv {
            debug!(
                "Scanning {} for implicit dependency relations",
                color::id(project_id),
            );

            // TODO: support parsing `tool.uv` sections
            if let Some(data) =
                uv::PyProjectTomlCache::read(self.workspace_root.join(project_source))?
            {
                if let Some(project) = data.project {
                    if let Some(deps) = project.dependencies {
                        for dep in deps {
                            let dep_name = dep.name.to_string();

                            if dep.extras.is_empty()
                                && dep.version_or_url.is_none()
                                && dep.origin.is_none()
                            {
                                if let Some(dep_project_id) = self.package_names.get(&dep_name) {
                                    implicit_deps.push(DependencyConfig {
                                        id: dep_project_id.to_owned(),
                                        scope: DependencyScope::Production,
                                        source: DependencySource::Implicit,
                                        via: Some(dep_name.clone()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(implicit_deps)
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
        let tool = self.toolchain.get()?;
        let depman = tool.get_package_manager();

        Ok(Some((
            depman.get_lock_filename(),
            depman.get_manifest_filename(),
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

        Ok(installed)
    }

    #[instrument(skip_all)]
    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<Vec<Operation>> {
        actions::install_deps(
            self.toolchain.get_for_version(&runtime.requirement)?,
            self.workspace_root.as_path(),
            working_dir,
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
        Ok(false)
    }

    #[instrument(skip_all)]
    async fn hash_manifest_deps(
        &self,
        manifest_path: &Path,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        match self.config.package_manager {
            PythonPackageManager::Pip => {
                if let Ok(data) = pip::load_lockfile_dependencies(manifest_path.to_path_buf()) {
                    let mut hash = DepsHash::new("unknown".into());
                    let mut project_deps = BTreeMap::default();

                    for (key, req) in data {
                        project_deps.insert(key, req.join(""));
                    }

                    hash.add_deps(&project_deps);
                    hasher.hash_content(hash)?;
                }
            }
            PythonPackageManager::Uv => {
                if let Some(data) = uv::PyProjectTomlCache::read(manifest_path)? {
                    if let Some(project) = data.project {
                        let mut hash = DepsHash::new(project.name);
                        let mut project_deps = BTreeMap::default();

                        if let Some(deps) = project.dependencies {
                            for dep in deps {
                                project_deps.insert(dep.name.to_string(), dep.to_string());
                            }
                        }

                        hash.add_deps(&project_deps);
                        hasher.hash_content(hash)?;
                    }
                }
            }
        };

        Ok(())
    }

    #[instrument(skip_all)]
    async fn hash_run_target(
        &self,
        project: &Project,
        runtime: &Runtime,
        hasher: &mut ContentHasher,
        hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        let python_tool = self.toolchain.get_for_version(&runtime.requirement).ok();
        let mut content = PythonToolchainHash {
            version: self
                .config
                .version
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            dependencies: BTreeMap::new(),
        };

        let resolved_dependencies =
            if matches!(hasher_config.optimization, HasherOptimization::Accuracy)
                && python_tool.is_some()
            {
                python_tool
                    .unwrap()
                    .get_package_manager()
                    .get_resolved_dependencies(&project.root)
                    .await?
            } else {
                FxHashMap::default()
            };

        match self.config.package_manager {
            // Since the manifest and lockfile are the same (requirements.txt),
            // just inherit the resolved dependencies as-is
            PythonPackageManager::Pip => {
                content.dependencies.extend(resolved_dependencies);
            }
            PythonPackageManager::Uv => {
                if let Some(data) = uv::PyProjectTomlCache::read(&project.root)? {
                    if let Some(project) = data.project {
                        if let Some(deps) = project.dependencies {
                            for dep in deps {
                                let name = dep.name.to_string();

                                if let Some(resolved_versions) = resolved_dependencies.get(&name) {
                                    let mut sorted_deps = resolved_versions.to_owned().clone();
                                    sorted_deps.sort();
                                    content.dependencies.insert(name, sorted_deps);
                                } else {
                                    content.dependencies.insert(name, vec![dep.to_string()]);
                                }
                            }
                        }
                    }
                }
            }
        };

        hasher.hash_content(content)?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn create_run_target_command(
        &self,
        _context: &ActionContext,
        _project: &Project,
        task: &Task,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<Command> {
        let mut command = Command::new(&task.command);

        command.with_console(self.console.clone());
        command.args(&task.args);
        command.envs(&task.env);

        if let Ok(python) = self.toolchain.get_for_version(&runtime.requirement) {
            if let Some(version) = get_proto_version_env(&python.tool) {
                command.env("PROTO_PYTHON_VERSION", version);
            }

            command.env(
                "PATH",
                prepend_path_env_var(get_python_tool_paths(
                    python,
                    working_dir,
                    &self.workspace_root,
                )),
            );
        }

        Ok(command)
    }
}
