use crate::errors::ProjectError;
use crate::file_group::FileGroup;
use crate::target::Target;
use crate::task::Task;
use crate::token::{TokenResolver, TokenSharedData};
use crate::types::TouchedFilePaths;
use moon_config::constants::CONFIG_PROJECT_FILENAME;
use moon_config::package::PackageJson;
use moon_config::tsconfig::TsConfigJson;
use moon_config::{FilePath, GlobalProjectConfig, ProjectConfig, ProjectID, TaskID};
use moon_logger::{color, debug, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub type FileGroupsMap = HashMap<String, FileGroup>;

pub type ProjectsMap = HashMap<ProjectID, Project>;

pub type TasksMap = HashMap<TaskID, Task>;

// project.yml
fn load_project_config(
    workspace_root: &Path,
    project_source: &str,
) -> Result<Option<ProjectConfig>, ProjectError> {
    let config_path = workspace_root
        .join(&project_source)
        .join(CONFIG_PROJECT_FILENAME);

    trace!(
        target: "moon:project",
        "Attempting to find {} in {}",
        color::path("project.yml"),
        color::file_path(&workspace_root.join(&project_source)),
    );

    if config_path.exists() {
        return match ProjectConfig::load(&config_path) {
            Ok(cfg) => Ok(Some(cfg)),
            Err(error) => Err(ProjectError::InvalidConfigFile(
                String::from(project_source),
                error,
            )),
        };
    }

    Ok(None)
}

fn create_file_groups_from_config(
    config: &Option<ProjectConfig>,
    global_config: &GlobalProjectConfig,
) -> FileGroupsMap {
    let mut file_groups = HashMap::<String, FileGroup>::new();

    // Add global file groups first
    if let Some(global_file_groups) = &global_config.file_groups {
        for (group_id, files) in global_file_groups {
            file_groups.insert(
                group_id.to_owned(),
                FileGroup::new(group_id, files.to_owned()),
            );
        }
    }

    // Override global configs with local
    if let Some(local_config) = config {
        if let Some(local_file_groups) = &local_config.file_groups {
            for (group_id, files) in local_file_groups {
                if file_groups.contains_key(group_id) {
                    // Group already exists, so merge with it
                    file_groups
                        .get_mut(group_id)
                        .unwrap()
                        .merge(files.to_owned());
                } else {
                    // Insert a group
                    file_groups
                        .insert(group_id.clone(), FileGroup::new(group_id, files.to_owned()));
                }
            }
        }
    }

    file_groups
}

fn create_tasks_from_config(
    config: &Option<ProjectConfig>,
    global_config: &GlobalProjectConfig,
    workspace_root: &Path,
    project_root: &Path,
    project_id: &str,
    file_groups: &FileGroupsMap,
) -> Result<TasksMap, ProjectError> {
    let mut tasks = HashMap::<String, Task>::new();

    // Add global tasks first
    if let Some(global_tasks) = &global_config.tasks {
        for (task_id, task_config) in global_tasks {
            tasks.insert(
                task_id.clone(),
                Task::from_config(Target::format(project_id, task_id)?, task_config),
            );
        }
    }

    // Add local tasks second
    if let Some(local_config) = config {
        if let Some(local_tasks) = &local_config.tasks {
            for (task_id, task_config) in local_tasks {
                if tasks.contains_key(task_id) {
                    // Task already exists, so merge with it
                    tasks.get_mut(task_id).unwrap().merge(task_config);
                } else {
                    // Insert a new task
                    tasks.insert(
                        task_id.clone(),
                        Task::from_config(Target::format(project_id, task_id)?, task_config),
                    );
                }
            }
        }
    }

    // Expand args, inputs, and outputs after all tasks have been created
    for task in tasks.values_mut() {
        let data = TokenSharedData::new(file_groups, workspace_root, project_root);

        task.expand_inputs(TokenResolver::for_inputs(&data))?;
        task.expand_outputs(TokenResolver::for_outputs(&data))?;

        // Must be last as it references inputs/outputs
        task.expand_args(TokenResolver::for_args(&data))?;
    }

    Ok(tasks)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Project configuration loaded from "project.yml", if it exists.
    pub config: Option<ProjectConfig>,

    /// File groups specific to the project. Inherits all file groups from the global config.
    pub file_groups: FileGroupsMap,

    /// Unique ID for the project. Is the LHS of the `projects` setting.
    pub id: ProjectID,

    /// Absolute path to the project's root folder.
    pub root: PathBuf,

    /// Relative path of the project from the workspace root. Is the RHS of the `projects` setting.
    pub source: FilePath,

    /// Tasks specific to the project. Inherits all tasks from the global config.
    pub tasks: TasksMap,
}

impl Project {
    pub fn new(
        id: &str,
        source: &str,
        workspace_root: &Path,
        global_config: &GlobalProjectConfig,
    ) -> Result<Project, ProjectError> {
        let root = workspace_root.join(&source);

        debug!(
            target: "moon:project",
            "Loading project from {} (id = {}, path = {})",
            color::file_path(&root),
            color::id(id),
            color::path(source),
        );

        if !root.exists() {
            return Err(ProjectError::MissingProject(String::from(source)));
        }

        let root = root.canonicalize().unwrap();
        let config = load_project_config(workspace_root, source)?;
        let file_groups = create_file_groups_from_config(&config, global_config);
        let tasks = create_tasks_from_config(
            &config,
            global_config,
            workspace_root,
            &root,
            id,
            &file_groups,
        )?;

        Ok(Project {
            config,
            file_groups,
            id: String::from(id),
            root,
            source: String::from(source),
            tasks,
        })
    }

    /// Return a list of project IDs this project depends on.
    pub fn get_dependencies(&self) -> Vec<ProjectID> {
        let mut depends_on = vec![];

        if let Some(config) = &self.config {
            if let Some(config_depends) = &config.depends_on {
                depends_on.extend_from_slice(config_depends);
            }
        }

        depends_on.sort();

        depends_on
    }

    /// Return the "package.json" name, if the file exists.
    pub async fn get_package_name(&self) -> Result<Option<String>, ProjectError> {
        if let Some(json) = self.load_package_json().await? {
            if let Some(name) = &json.name {
                return Ok(Some(name.clone()));
            }
        }

        Ok(None)
    }

    /// Return a task with the defined ID.
    pub fn get_task(&self, task_id: &str) -> Result<&Task, ProjectError> {
        match self.tasks.get(task_id) {
            Some(t) => Ok(t),
            None => Err(ProjectError::UnconfiguredTask(
                task_id.to_owned(),
                self.id.to_owned(),
            )),
        }
    }

    /// Return true if this project is affected, based on touched files.
    /// Will attempt to find any file that starts with the project root.
    pub fn is_affected(&self, touched_files: &TouchedFilePaths) -> bool {
        for file in touched_files {
            if file.starts_with(&self.root) {
                return true;
            }
        }

        false
    }

    /// Load and parse the package's `package.json` if it exists.
    pub async fn load_package_json(&self) -> Result<Option<PackageJson>, ProjectError> {
        let package_path = self.root.join("package.json");

        trace!(
            target: "moon:project",
            "Attempting to find {} in {}",
            color::path("package.json"),
            color::file_path(&self.root),
        );

        if package_path.exists() {
            return match PackageJson::load(&package_path).await {
                Ok(json) => Ok(Some(json)),
                Err(error) => Err(ProjectError::InvalidPackageJson(
                    String::from(&self.source),
                    error.to_string(),
                )),
            };
        }

        Ok(None)
    }

    /// Load and parse the package's `tsconfig.json` if it exists.
    pub async fn load_tsconfig_json(
        &self,
        tsconfig_name: &str,
    ) -> Result<Option<TsConfigJson>, ProjectError> {
        let tsconfig_path = self.root.join(tsconfig_name);

        trace!(
            target: "moon:project",
            "Attempting to find {} in {}",
            color::path(tsconfig_name),
            color::file_path(&self.root),
        );

        if tsconfig_path.exists() {
            return match TsConfigJson::load(&tsconfig_path).await {
                Ok(cfg) => Ok(Some(cfg)),
                Err(error) => Err(ProjectError::InvalidTsConfigJson(
                    String::from(&self.source),
                    String::from(tsconfig_name),
                    error.to_string(),
                )),
            };
        }

        Ok(None)
    }

    /// Return the project as a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_config::GlobalProjectConfig;
    use moon_utils::test::get_fixtures_root;
    use std::collections::HashSet;

    mod is_affected {
        use super::*;

        #[test]
        fn returns_true_if_inside_project() {
            let root = get_fixtures_root();
            let project = Project::new(
                "basic",
                "projects/basic",
                &root,
                &GlobalProjectConfig::default(),
            )
            .unwrap();

            let mut set = HashSet::new();
            set.insert(root.join("projects/basic/file.ts"));

            assert!(project.is_affected(&set));
        }

        #[test]
        fn returns_false_if_outside_project() {
            let root = get_fixtures_root();
            let project = Project::new(
                "basic",
                "projects/basic",
                &root,
                &GlobalProjectConfig::default(),
            )
            .unwrap();

            let mut set = HashSet::new();
            set.insert(root.join("projects/other/file.ts"));

            assert!(!project.is_affected(&set));
        }
    }
}
