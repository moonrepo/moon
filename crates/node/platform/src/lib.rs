pub mod actions;
mod hasher;
pub mod task;

pub use hasher::NodeTargetHasher;
use moon_config::{
    DependencyConfig, DependencyScope, NodeConfig, NodeProjectAliasFormat, PlatformType,
    ProjectConfig, ProjectID, ProjectLanguage, ProjectsAliasesMap, ProjectsSourcesMap,
    TasksConfigsMap,
};
use moon_error::MoonError;
use moon_logger::{color, debug, warn};
use moon_node_lang::node::{get_package_manager_workspaces, parse_package_name};
use moon_node_lang::{PackageJson, NPM};
use moon_platform::{Platform, Runtime, Version};
use moon_task::TaskError;
use moon_utils::glob::GlobSet;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::{collections::BTreeMap, path::Path};
use task::ScriptParser;

pub const LOG_TARGET: &str = "moon:node-platform";

pub fn create_tasks_from_scripts(
    project_id: &str,
    package_json: &mut PackageJson,
) -> Result<TasksConfigsMap, TaskError> {
    let mut parser = ScriptParser::new(project_id);

    parser.parse_scripts(package_json)?;
    parser.update_package(package_json)?;

    Ok(parser.tasks)
}

pub fn infer_tasks_from_scripts(
    project_id: &str,
    package_json: &PackageJson,
) -> Result<TasksConfigsMap, TaskError> {
    let mut parser = ScriptParser::new(project_id);

    parser.infer_scripts(package_json)?;

    Ok(parser.tasks)
}

#[derive(Debug)]
pub struct NodePlatform {
    config: NodeConfig,

    /// Maps `package.json` names to project IDs.
    package_names: FxHashMap<String, ProjectID>,

    workspace_root: PathBuf,
}

impl NodePlatform {
    pub fn new(config: &NodeConfig, workspace_root: &Path) -> Self {
        NodePlatform {
            config: config.to_owned(),
            package_names: FxHashMap::default(),
            workspace_root: workspace_root.to_path_buf(),
        }
    }
}

impl Platform for NodePlatform {
    fn detect_project_language(&self, project_root: &Path) -> Option<ProjectLanguage> {
        if project_root.join("tsconfig.json").exists() {
            Some(ProjectLanguage::TypeScript)
        } else if project_root.join("package.json").exists() {
            Some(ProjectLanguage::JavaScript)
        } else {
            None
        }
    }

    fn get_type(&self) -> PlatformType {
        PlatformType::Node
    }

    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Option<Runtime> {
        if let Some(config) = &project_config {
            if let Some(node_config) = &config.toolchain.node {
                if let Some(version) = &node_config.version {
                    return Some(Runtime::Node(Version(version.to_owned(), true)));
                }
            }
        }

        Some(Runtime::Node(Version(
            self.config.version.to_owned(),
            false,
        )))
    }

    fn is_project_in_dependency_workspace(
        &self,
        project_id: &str,
        project_root: &Path,
    ) -> Result<bool, MoonError> {
        let mut in_workspace = false;

        // Root package is always considered within the workspace
        if project_root == self.workspace_root {
            return Ok(true);
        }

        if let Some(globs) = get_package_manager_workspaces(self.workspace_root.to_owned())? {
            in_workspace = GlobSet::new(globs)
                .map_err(|e| MoonError::Generic(e.to_string()))?
                .matches(project_root.strip_prefix(&self.workspace_root).unwrap())?;
        }

        if !in_workspace {
            debug!(
                target: LOG_TARGET,
                "Project {} not within root {} workspaces, will be handled externally",
                color::id(project_id),
                color::file(NPM.manifest_filename)
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
            color::file(NPM.manifest_filename)
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
        project_id: &str,
        project_root: &Path,
        _project_config: &ProjectConfig,
        _aliases_map: &ProjectsAliasesMap,
    ) -> Result<Vec<DependencyConfig>, MoonError> {
        let mut implicit_deps = vec![];

        debug!(
            target: LOG_TARGET,
            "Scanning {} for implicit dependency relations",
            color::id(project_id),
        );

        if let Some(package_json) = PackageJson::read(project_root)? {
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

    fn load_project_tasks(
        &self,
        project_id: &str,
        project_root: &Path,
        _project_config: &ProjectConfig,
    ) -> Result<TasksConfigsMap, MoonError> {
        let mut tasks = BTreeMap::new();

        if !self.config.infer_tasks_from_scripts {
            return Ok(tasks);
        }

        debug!(
            target: LOG_TARGET,
            "Inferring {} tasks from {}",
            color::id(project_id),
            color::file(NPM.manifest_filename)
        );

        if let Some(package_json) = PackageJson::read(project_root)? {
            tasks.extend(
                infer_tasks_from_scripts(project_id, &package_json)
                    .map_err(|e| MoonError::Generic(e.to_string()))?,
            );
        }

        Ok(tasks)
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
}
