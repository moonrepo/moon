use crate::actions;
use crate::infer_tasks_from_scripts;
use moon_action_context::ActionContext;
use moon_common::Id;
use moon_config::{
    Config, DependencyConfig, DependencyScope, DependencySource, HasherConfig, NodeConfig,
    PlatformType, ProjectConfig, ProjectsAliasesMap, ProjectsSourcesMap, TaskConfig,
    TasksConfigsMap, TypeScriptConfig,
};
use moon_hash::{ContentHasher, DepsHash};
use moon_logger::{debug, warn};
use moon_node_lang::node::get_package_manager_workspaces;
use moon_node_lang::{PackageJson, NPM};
use moon_node_tool::NodeTool;
use moon_platform::{Platform, Runtime, Version};
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_tool::{Tool, ToolManager};
use moon_typescript_platform::TypeScriptTargetHash;
use moon_utils::async_trait;
use proto_core::{PluginLoader, ProtoEnvironment};
use rustc_hash::FxHashMap;
use starbase_styles::color;
use starbase_utils::glob::GlobSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::BTreeMap, path::Path};

const LOG_TARGET: &str = "moon:node-platform";

pub struct NodePlatform {
    config: NodeConfig,

    package_names: FxHashMap<String, Id>,

    toolchain: ToolManager<NodeTool>,

    typescript_config: Option<TypeScriptConfig>,

    workspace_root: PathBuf,
}

impl NodePlatform {
    pub fn new(
        config: &NodeConfig,
        typescript_config: &Option<TypeScriptConfig>,
        workspace_root: &Path,
    ) -> Self {
        NodePlatform {
            config: config.to_owned(),
            package_names: FxHashMap::default(),
            toolchain: ToolManager::new(Runtime::Node(Version::new_global())),
            typescript_config: typescript_config.to_owned(),
            workspace_root: workspace_root.to_path_buf(),
        }
    }
}

#[async_trait]
impl Platform for NodePlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::Node
    }

    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Runtime {
        if let Some(config) = &project_config {
            if let Some(node_config) = &config.toolchain.node {
                if let Some(version) = &node_config.version {
                    return Runtime::Node(Version::new_override(version));
                }
            }
        }

        if let Some(version) = &self.config.version {
            return Runtime::Node(Version::new(version));
        }

        // Global
        Runtime::Node(Version::new_global())
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::Node) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime, Runtime::Node(_));
        }

        false
    }

    // PROJECT GRAPH

    fn is_project_in_dependency_workspace(&self, project_source: &str) -> miette::Result<bool> {
        let mut in_workspace = false;

        // Root package is always considered within the workspace
        if project_source.is_empty() || project_source == "." {
            return Ok(true);
        }

        if let Some(globs) = get_package_manager_workspaces(self.workspace_root.to_owned())? {
            in_workspace = GlobSet::new(&globs)?.matches(project_source);
        }

        Ok(in_workspace)
    }

    fn load_project_graph_aliases(
        &mut self,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> miette::Result<()> {
        debug!(
            target: LOG_TARGET,
            "Loading names (aliases) from project {}'s",
            color::file(NPM.manifest)
        );

        for (project_id, project_source) in projects_map {
            if let Some(package_json) =
                PackageJson::read(project_source.to_path(&self.workspace_root))?
            {
                if let Some(package_name) = package_json.name {
                    let alias = package_name.clone();

                    self.package_names
                        .insert(package_name.clone(), project_id.to_owned());

                    if let Some(existing_source) = projects_map.get(&alias) {
                        if existing_source != project_source {
                            warn!(
                                target: LOG_TARGET,
                                "A project already exists with the ID {} ({}), skipping alias of the same name ({})",
                                color::id(&alias),
                                color::file(existing_source),
                                color::file(project_source)
                            );

                            continue;
                        }
                    }

                    if let Some(existing_id) = aliases_map.get(&alias) {
                        warn!(
                            target: LOG_TARGET,
                            "A project already exists with the alias {} (for ID {}), skipping conflicting alias (from {})",
                            color::id(alias),
                            color::id(existing_id),
                            color::file(project_source)
                        );

                        continue;
                    }

                    aliases_map.insert(alias, project_id.to_owned());
                }
            }
        }

        Ok(())
    }

    fn load_project_implicit_dependencies(
        &self,
        project_id: &str,
        project_source: &str,
    ) -> miette::Result<Vec<DependencyConfig>> {
        let mut implicit_deps = vec![];

        debug!(
            target: LOG_TARGET,
            "Scanning {} for implicit dependency relations",
            color::id(project_id),
        );

        if let Some(package_json) = PackageJson::read(self.workspace_root.join(project_source))? {
            let mut find_implicit_relations =
                |package_deps: &BTreeMap<String, String>, scope: &DependencyScope| {
                    for dep_name in package_deps.keys() {
                        if let Some(dep_project_id) = self.package_names.get(dep_name) {
                            implicit_deps.push(DependencyConfig {
                                id: dep_project_id.to_owned(),
                                scope: *scope,
                                source: DependencySource::Implicit,
                                via: Some(dep_name.clone()),
                            });
                        }
                    }
                };

            if let Some(dependencies) = &package_json.dependencies {
                find_implicit_relations(dependencies, &DependencyScope::Production);
            }

            if let Some(dev_dependencies) = &package_json.dev_dependencies {
                find_implicit_relations(dev_dependencies, &DependencyScope::Development);
            }

            if let Some(peer_dependencies) = &package_json.peer_dependencies {
                find_implicit_relations(peer_dependencies, &DependencyScope::Peer);
            }
        }

        Ok(implicit_deps)
    }

    fn load_project_tasks(
        &self,
        project_id: &str,
        project_source: &str,
    ) -> miette::Result<TasksConfigsMap> {
        let mut tasks = BTreeMap::new();

        if !self.config.infer_tasks_from_scripts {
            return Ok(tasks);
        }

        debug!(
            target: LOG_TARGET,
            "Inferring {} tasks from {}",
            color::id(project_id),
            color::file(NPM.manifest)
        );

        if let Some(package_json) = PackageJson::read(self.workspace_root.join(project_source))? {
            for (id, partial_task) in infer_tasks_from_scripts(project_id, &package_json)? {
                tasks.insert(id, TaskConfig::from_partial(partial_task));
            }
        }

        Ok(tasks)
    }

    // TOOLCHAIN

    fn is_toolchain_enabled(&self) -> miette::Result<bool> {
        Ok(self.config.version.is_some())
    }

    fn get_tool(&self) -> miette::Result<Box<&dyn Tool>> {
        let tool = self.toolchain.get()?;

        Ok(Box::new(tool))
    }

    fn get_tool_for_version(&self, version: Version) -> miette::Result<Box<&dyn Tool>> {
        let tool = self.toolchain.get_for_version(&version)?;

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

    async fn setup_toolchain(&mut self, plugin_loader: &PluginLoader) -> miette::Result<()> {
        let version = match &self.config.version {
            Some(v) => Version::new(v),
            None => Version::new_global(),
        };

        let mut last_versions = FxHashMap::default();

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                NodeTool::new(
                    &ProtoEnvironment::new()?,
                    &self.config,
                    &version,
                    plugin_loader,
                )
                .await?,
            );
        }

        self.toolchain.setup(&version, &mut last_versions).await?;

        Ok(())
    }

    async fn teardown_toolchain(&mut self) -> miette::Result<()> {
        self.toolchain.teardown_all().await?;

        Ok(())
    }

    // ACTIONS

    async fn setup_tool(
        &mut self,
        _context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, String>,
        plugin_loader: &PluginLoader,
    ) -> miette::Result<u8> {
        let version = runtime.version();

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                NodeTool::new(
                    &ProtoEnvironment::new()?,
                    &self.config,
                    &version,
                    plugin_loader,
                )
                .await?,
            );
        }

        let installed = self.toolchain.setup(&version, last_versions).await?;

        actions::setup_tool(
            self.toolchain.get_for_version(runtime.version())?,
            &self.workspace_root,
        )
        .await?;

        Ok(installed)
    }

    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<()> {
        actions::install_deps(
            self.toolchain.get_for_version(runtime.version())?,
            working_dir,
        )
        .await?;

        Ok(())
    }

    async fn sync_project(
        &self,
        _context: &ActionContext,
        project: &Project,
        dependencies: &FxHashMap<Id, Arc<Project>>,
    ) -> miette::Result<bool> {
        let modified = actions::sync_project(
            project,
            dependencies,
            &self.workspace_root,
            &self.config,
            &self.typescript_config,
        )
        .await?;

        Ok(modified)
    }

    async fn hash_manifest_deps(
        &self,
        manifest_path: &Path,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        if let Ok(Some(package)) = PackageJson::read(manifest_path) {
            let name = package.name.unwrap_or_else(|| "unknown".into());
            let mut hash = DepsHash::new(name);

            if let Some(peer_deps) = &package.peer_dependencies {
                hash.add_deps(peer_deps);
            }

            if let Some(dev_deps) = &package.dev_dependencies {
                hash.add_deps(dev_deps);
            }

            if let Some(deps) = &package.dependencies {
                hash.add_deps(deps);
            }

            hasher.hash_content(hash)?;
        }

        Ok(())
    }

    async fn hash_run_target(
        &self,
        project: &Project,
        runtime: &Runtime,
        hasher: &mut ContentHasher,
        hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        let node_hash = actions::create_target_hasher(
            self.toolchain.get_for_version(runtime.version()).ok(),
            project,
            &self.workspace_root,
            hasher_config,
        )
        .await?;

        hasher.hash_content(node_hash)?;

        if let Some(typescript_config) = &self.typescript_config {
            let ts_hash = TypeScriptTargetHash::generate(
                typescript_config,
                &self.workspace_root,
                &project.root,
            )?;

            hasher.hash_content(ts_hash)?;
        }

        Ok(())
    }

    async fn create_run_target_command(
        &self,
        context: &ActionContext,
        project: &Project,
        task: &Task,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<Command> {
        let command = if self.is_toolchain_enabled()? {
            actions::create_target_command(
                self.toolchain.get_for_version(runtime.version())?,
                context,
                project,
                task,
                working_dir,
            )?
        } else {
            actions::create_target_command_without_tool(
                &self.config,
                context,
                project,
                task,
                working_dir,
            )?
        };

        Ok(command)
    }
}
