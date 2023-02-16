use crate::actions;
use crate::infer_tasks_from_scripts;
use moon_action_context::ActionContext;
use moon_config::{
    DependencyConfig, DependencyScope, HasherConfig, NodeConfig, NodeProjectAliasFormat,
    PlatformType, ProjectConfig, ProjectID, ProjectsAliasesMap, ProjectsSourcesMap,
    TasksConfigsMap, TypeScriptConfig,
};
use moon_error::MoonError;
use moon_hasher::{DepsHasher, HashSet};
use moon_logger::{color, debug, warn};
use moon_node_lang::node::{get_package_manager_workspaces, parse_package_name};
use moon_node_lang::{PackageJson, NPM};
use moon_node_tool::NodeTool;
use moon_platform::{Platform, Runtime, Version};
use moon_project::{Project, ProjectError};
use moon_task::Task;
use moon_tool::{Tool, ToolError, ToolManager};
use moon_utils::{async_trait, glob::GlobSet, process::Command};
use proto::Proto;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::{collections::BTreeMap, path::Path};

const LOG_TARGET: &str = "moon:node-platform";

#[derive(Debug)]
pub struct NodePlatform {
    config: NodeConfig,

    package_names: FxHashMap<String, ProjectID>,

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

        if let Some(node_version) = &self.config.version {
            return Runtime::Node(Version::new(node_version));
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

    fn is_project_in_dependency_workspace(&self, project: &Project) -> Result<bool, MoonError> {
        let mut in_workspace = false;

        // Root package is always considered within the workspace
        if project.root == self.workspace_root {
            return Ok(true);
        }

        if let Some(globs) = get_package_manager_workspaces(self.workspace_root.to_owned())? {
            in_workspace = GlobSet::new(globs, vec![])
                .map_err(|e| MoonError::Generic(e.to_string()))?
                .matches(project.root.strip_prefix(&self.workspace_root).unwrap());
        }

        if !in_workspace {
            debug!(
                target: LOG_TARGET,
                "Project {} not within root {} workspaces, will be handled externally",
                color::id(&project.id),
                color::file(NPM.manifest)
            );
        }

        Ok(in_workspace)
    }

    fn load_project_graph_aliases(
        &mut self,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> Result<(), MoonError> {
        let mut map_aliases = false;
        let mut alias_format = NodeProjectAliasFormat::NameAndScope;

        if let Some(custom_format) = &self.config.alias_package_names {
            map_aliases = true;
            alias_format = custom_format.clone();
        }

        debug!(
            target: LOG_TARGET,
            "Loading names (aliases) from project {}'s",
            color::file(NPM.manifest)
        );

        for (project_id, project_source) in projects_map {
            if let Some(package_json) = PackageJson::read(self.workspace_root.join(project_source))?
            {
                if let Some(package_name) = package_json.name {
                    // Always track package names internally so that we can discover implicit dependencies
                    self.package_names
                        .insert(package_name.clone(), project_id.to_owned());

                    // However, consumers using aliases is opt-in, so account for that
                    if !map_aliases {
                        continue;
                    }

                    let mut aliases = vec![];

                    // We need to support both formats regardless of what the setting is.
                    // The setting just allows consumers to use a shorthand in addition
                    // to the full original name!
                    match alias_format {
                        NodeProjectAliasFormat::NameAndScope => {
                            aliases.push(package_name.clone());
                        }
                        NodeProjectAliasFormat::NameOnly => {
                            let name_only = parse_package_name(&package_name).1;

                            if name_only == package_name {
                                aliases.push(name_only);
                            } else {
                                aliases.push(name_only);
                                aliases.push(package_name.clone());
                            }
                        }
                    };

                    for alias in aliases {
                        if let Some(existing_source) = projects_map.get(&alias) {
                            if existing_source != project_source {
                                warn!(
                                target: LOG_TARGET,
                                "A project already exists with the ID {} ({}), skipping alias of the same name ({})",
                                color::id(alias),
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
        }

        Ok(())
    }

    fn load_project_implicit_dependencies(
        &self,
        project: &Project,
        _aliases_map: &ProjectsAliasesMap,
    ) -> Result<Vec<DependencyConfig>, MoonError> {
        let mut implicit_deps = vec![];

        debug!(
            target: LOG_TARGET,
            "Scanning {} for implicit dependency relations",
            color::id(&project.id),
        );

        if let Some(package_json) = PackageJson::read(&project.root)? {
            let mut find_implicit_relations =
                |package_deps: &BTreeMap<String, String>, scope: &DependencyScope| {
                    for dep_name in package_deps.keys() {
                        if let Some(dep_project_id) = self.package_names.get(dep_name) {
                            implicit_deps.push(DependencyConfig {
                                id: dep_project_id.to_owned(),
                                scope: scope.clone(),
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

    fn load_project_tasks(&self, project: &Project) -> Result<TasksConfigsMap, MoonError> {
        let mut tasks = BTreeMap::new();

        if !self.config.infer_tasks_from_scripts {
            return Ok(tasks);
        }

        debug!(
            target: LOG_TARGET,
            "Inferring {} tasks from {}",
            color::id(&project.id),
            color::file(NPM.manifest)
        );

        if let Some(package_json) = PackageJson::read(&project.root)? {
            tasks.extend(
                infer_tasks_from_scripts(&project.id, &package_json)
                    .map_err(|e| MoonError::Generic(e.to_string()))?,
            );
        }

        Ok(tasks)
    }

    // TOOLCHAIN

    fn is_toolchain_enabled(&self) -> Result<bool, ToolError> {
        Ok(self.config.version.is_some())
    }

    fn get_tool(&self) -> Result<Box<&dyn Tool>, ToolError> {
        let tool = self.toolchain.get()?;

        Ok(Box::new(tool))
    }

    fn get_tool_for_version(&self, version: Version) -> Result<Box<&dyn Tool>, ToolError> {
        let tool = self.toolchain.get_for_version(&version)?;

        Ok(Box::new(tool))
    }

    fn get_dependency_configs(&self) -> Result<Option<(String, String)>, ToolError> {
        let tool = self.toolchain.get()?;
        let depman = tool.get_package_manager();

        Ok(Some((
            depman.get_lock_filename(),
            depman.get_manifest_filename(),
        )))
    }

    async fn setup_toolchain(&mut self) -> Result<(), ToolError> {
        let version = match &self.config.version {
            Some(v) => Version::new(v),
            None => Version::new_global(),
        };

        let mut last_versions = FxHashMap::default();

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                NodeTool::new(&Proto::new()?, &self.config, &version)?,
            );
        }

        self.toolchain.setup(&version, &mut last_versions).await?;

        Ok(())
    }

    async fn teardown_toolchain(&mut self) -> Result<(), ToolError> {
        self.toolchain.teardown_all().await?;

        Ok(())
    }

    // ACTIONS

    async fn setup_tool(
        &mut self,
        _context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        let version = runtime.version();

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                NodeTool::new(&Proto::new()?, &self.config, &version)?,
            );
        }

        Ok(self.toolchain.setup(&version, last_versions).await?)
    }

    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> Result<(), ToolError> {
        actions::install_deps(
            self.toolchain.get_for_version(runtime.version())?,
            working_dir,
            &self.workspace_root,
        )
        .await?;

        Ok(())
    }

    async fn sync_project(
        &self,
        _context: &ActionContext,
        project: &Project,
        dependencies: &FxHashMap<String, &Project>,
    ) -> Result<bool, ProjectError> {
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
        hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        if let Ok(Some(package)) = PackageJson::read(manifest_path) {
            let name = package.name.unwrap_or_else(|| "unknown".into());
            let mut hasher = DepsHasher::new(name);

            if let Some(peer_deps) = &package.peer_dependencies {
                hasher.hash_deps(peer_deps);
            }

            if let Some(dev_deps) = &package.dev_dependencies {
                hasher.hash_deps(dev_deps);
            }

            if let Some(deps) = &package.dependencies {
                hasher.hash_deps(deps);
            }

            hashset.hash(hasher);
        }

        Ok(())
    }

    async fn hash_run_target(
        &self,
        project: &Project,
        runtime: &Runtime,
        hashset: &mut HashSet,
        hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        let hasher = actions::create_target_hasher(
            self.toolchain.get_for_version(runtime.version()).ok(),
            project,
            &self.workspace_root,
            hasher_config,
            &self.typescript_config,
        )
        .await?;

        hashset.hash(hasher);

        Ok(())
    }

    async fn create_run_target_command(
        &self,
        context: &ActionContext,
        project: &Project,
        task: &Task,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> Result<Command, ToolError> {
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
