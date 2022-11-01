pub mod actions;
mod hasher;
pub mod task;

pub use hasher::NodeTargetHasher;
use moon_config::{
    DependencyConfig, DependencyScope, NodeProjectAliasFormat, ProjectConfig, ProjectID,
    ProjectsAliasesMap, ProjectsSourcesMap, TasksConfigsMap, WorkspaceConfig,
};
use moon_error::MoonError;
use moon_lang_node::node::{get_package_manager_workspaces, parse_package_name};
use moon_lang_node::package::PackageJson;
use moon_lang_node::NPM;
use moon_logger::{color, debug, warn};
use moon_platform::{Platform, Runtime};
use moon_task::TaskError;
use moon_utils::glob::GlobSet;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};
use task::ScriptParser;

pub const LOG_TARGET: &str = "moon:platform-node";

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

#[derive(Debug, Default)]
pub struct NodePlatform {
    /// Maps `package.json` names to project IDs.
    package_names: HashMap<String, ProjectID>,
}

impl Platform for NodePlatform {
    fn get_runtime_from_config(
        &self,
        project_config: &ProjectConfig,
        workspace_config: &WorkspaceConfig,
    ) -> Option<Runtime> {
        if let Some(node_config) = &project_config.workspace.node {
            if let Some(version) = &node_config.version {
                return Some(Runtime::Node(version.to_owned()));
            }
        }

        if let Some(node_config) = &workspace_config.node {
            return Some(Runtime::Node(node_config.version.to_owned()));
        }

        None
    }

    fn is_project_in_package_manager_workspace(
        &self,
        project_id: &str,
        project_root: &Path,
        workspace_root: &Path,
        _workspace_config: &WorkspaceConfig,
    ) -> Result<bool, MoonError> {
        let mut in_workspace = false;

        // Root package is always considered within the workspace
        if project_root == workspace_root {
            return Ok(true);
        }

        if let Some(globs) = get_package_manager_workspaces(workspace_root.to_owned())? {
            in_workspace = GlobSet::new(globs)
                .map_err(|e| MoonError::Generic(e.to_string()))?
                .matches(project_root.strip_prefix(workspace_root).unwrap())?;
        }

        if !in_workspace {
            debug!(
                target: LOG_TARGET,
                "Project {} not within root {} workspaces, will be handled externally",
                color::id(project_id),
                color::file(&NPM.manifest_filename)
            );
        }

        Ok(in_workspace)
    }

    fn load_project_graph_aliases(
        &mut self,
        workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> Result<(), MoonError> {
        let mut map_aliases = false;
        let mut alias_format = NodeProjectAliasFormat::NameAndScope;

        if let Some(node_config) = &workspace_config.node {
            if let Some(custom_format) = &node_config.alias_package_names {
                map_aliases = true;
                alias_format = custom_format.clone();
            }
        }

        debug!(
            target: LOG_TARGET,
            "Loading names (aliases) from project {}'s",
            color::file(&NPM.manifest_filename)
        );

        for (project_id, project_source) in projects_map {
            if let Some(package_json) = PackageJson::read(workspace_root.join(project_source))? {
                if let Some(package_name) = package_json.name {
                    // Always track package names internally so that we can discover implicit dependencies
                    self.package_names
                        .insert(package_name.clone(), project_id.to_owned());

                    // However, consumers using aliases is opt-in, so account for that
                    if !map_aliases {
                        continue;
                    }

                    let alias = match alias_format {
                        NodeProjectAliasFormat::NameAndScope => package_name.clone(),
                        NodeProjectAliasFormat::NameOnly => parse_package_name(&package_name).1,
                    };

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
        _workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
    ) -> Result<TasksConfigsMap, MoonError> {
        let mut tasks = BTreeMap::new();

        if let Some(node_config) = &workspace_config.node {
            if !node_config.infer_tasks_from_scripts {
                return Ok(tasks);
            }
        }

        debug!(
            target: LOG_TARGET,
            "Inferring {} tasks from {}",
            color::id(project_id),
            color::file(&NPM.manifest_filename)
        );

        if let Some(package_json) = PackageJson::read(project_root)? {
            tasks.extend(
                infer_tasks_from_scripts(project_id, &package_json)
                    .map_err(|e| MoonError::Generic(e.to_string()))?,
            );
        }

        Ok(tasks)
    }

    fn matches(&self, project_config: &ProjectConfig, runtime: Option<&Runtime>) -> bool {
        if project_config.language.is_node_platform() {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime, Runtime::Node(_));
        }

        false
    }
}
