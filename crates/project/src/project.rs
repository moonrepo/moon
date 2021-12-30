use crate::errors::ProjectError;
use crate::task::Task;
use moon_config::constants::CONFIG_PROJECT_FILENAME;
use moon_config::{
    FileGroups, FilePath, GlobalProjectConfig, PackageJson, PackageJsonValue, ProjectConfig,
    ProjectID,
};
use moon_logger::{color, debug, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub type ProjectsMap = HashMap<ProjectID, Project>;

pub type TasksMap = HashMap<String, Task>;

// project.yml
fn load_project_config(
    root_dir: &Path,
    project_path: &str,
) -> Result<Option<ProjectConfig>, ProjectError> {
    let config_path = root_dir.join(&project_path).join(CONFIG_PROJECT_FILENAME);

    trace!(
        target: "moon:project",
        "Attempting to find {} in {}",
        color::path("project.yml"),
        color::file_path(&root_dir.join(&project_path)),
    );

    if config_path.exists() {
        return match ProjectConfig::load(&config_path) {
            Ok(cfg) => Ok(Some(cfg)),
            Err(error) => Err(ProjectError::InvalidConfigFile(
                String::from(project_path),
                error,
            )),
        };
    }

    Ok(None)
}

// package.json
fn load_package_json(
    root_dir: &Path,
    project_path: &str,
) -> Result<Option<PackageJsonValue>, ProjectError> {
    let package_path = root_dir.join(&project_path).join("package.json");

    trace!(
        target: "moon:project",
        "Attempting to find {} in {}",
        color::path("package.json"),
        color::file_path(&root_dir.join(&project_path)),
    );

    if package_path.exists() {
        return match PackageJson::load(&package_path) {
            Ok(json) => Ok(Some(json)),
            Err(error) => Err(ProjectError::InvalidPackageJson(
                String::from(project_path),
                error.to_string(),
            )),
        };
    }

    Ok(None)
}

fn create_file_groups_from_config(
    config: &Option<ProjectConfig>,
    global_config: &GlobalProjectConfig,
) -> FileGroups {
    let mut file_groups = global_config.file_groups.clone();

    // Override global configs with local
    if let Some(local_config) = config {
        if let Some(local_file_groups) = &local_config.file_groups {
            file_groups.extend(local_file_groups.clone());
        }
    }

    file_groups
}

fn create_tasks_from_config(
    config: &Option<ProjectConfig>,
    global_config: &GlobalProjectConfig,
) -> TasksMap {
    let mut tasks = HashMap::<String, Task>::new();

    // Add global tasks first
    if let Some(global_tasks) = &global_config.tasks {
        for (name, task_config) in global_tasks {
            tasks.insert(name.clone(), Task::from_config(name, task_config));
        }
    }

    // Add local tasks second
    if let Some(local_config) = config {
        if let Some(local_tasks) = &local_config.tasks {
            for (name, task_config) in local_tasks {
                if tasks.contains_key(name) {
                    // Task already exists, so merge with it
                    tasks.get_mut(name).unwrap().merge(task_config);
                } else {
                    // Insert a new task
                    tasks.insert(name.clone(), Task::from_config(name, task_config));
                }
            }
        }
    }

    tasks
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Project {
    /// Project configuration loaded from "project.yml", if it exists.
    pub config: Option<ProjectConfig>,

    /// Absolute path to the project's root folder.
    pub dir: PathBuf,

    /// File groups specific to the project. Inherits all file groups from the global config.
    #[serde(rename = "fileGroups")]
    pub file_groups: FileGroups,

    /// Unique ID for the project. Is the LHS of the `projects` setting.
    pub id: ProjectID,

    /// Relative path of the project from the workspace root. Is the RHS of the `projects` setting.
    pub location: FilePath,

    /// Loaded "package.json", if it exists.
    #[serde(skip)]
    pub package_json: Option<PackageJsonValue>,

    /// Tasks specific to the project. Inherits all tasks from the global config.
    pub tasks: TasksMap,
}

impl Project {
    pub fn new(
        id: &str,
        location: &str,
        root_dir: &Path,
        global_config: &GlobalProjectConfig,
    ) -> Result<Project, ProjectError> {
        let dir = root_dir.join(&location);

        debug!(
            target: "moon:project",
            "Loading project from {} (id = {}, path = {})",
            color::file_path(&dir),
            color::id(id),
            color::path(location),
        );

        if !dir.exists() {
            return Err(ProjectError::MissingFilePath(String::from(location)));
        }

        let config = load_project_config(root_dir, location)?;
        let package_json = load_package_json(root_dir, location)?;
        let file_groups = create_file_groups_from_config(&config, global_config);
        let tasks = create_tasks_from_config(&config, global_config);

        Ok(Project {
            config,
            dir: dir.canonicalize().unwrap(),
            file_groups,
            id: String::from(id),
            location: String::from(location),
            package_json,
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

    /// Return the project as a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
