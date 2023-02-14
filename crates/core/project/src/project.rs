use crate::errors::ProjectError;
use moon_config::{
    format_error_line, format_figment_errors, ConfigError, DependencyConfig, DependencyScope,
    FilePath, InheritedTasksConfig, InheritedTasksManager, ProjectConfig, ProjectDependsOn,
    ProjectID, ProjectLanguage, ProjectType, TaskID,
};
use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_logger::{color, debug, trace, Logable};
use moon_target::Target;
use moon_task::{FileGroup, Task, TouchedFilePaths};
use moon_utils::path;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use strum::Display;

type FileGroupsMap = FxHashMap<String, FileGroup>;

type ProjectDependenciesMap = FxHashMap<ProjectID, ProjectDependency>;

type TasksMap = BTreeMap<TaskID, Task>;

// moon.yml
fn load_project_config(
    log_target: &str,
    project_root: &Path,
    project_source: &str,
) -> Result<ProjectConfig, ProjectError> {
    let config_path = project_root.join(CONFIG_PROJECT_FILENAME);

    trace!(
        target: log_target,
        "Attempting to find {} in {}",
        color::file(CONFIG_PROJECT_FILENAME),
        color::path(project_root),
    );

    if config_path.exists() {
        return ProjectConfig::load(config_path).map_err(|e| {
            ProjectError::InvalidConfigFile(
                String::from(project_source),
                if let ConfigError::FailedValidation(valids) = e {
                    format_figment_errors(valids)
                } else {
                    format_error_line(e.to_string())
                },
            )
        });
    }

    Ok(ProjectConfig::default())
}

fn create_file_groups_from_config(
    log_target: &str,
    config: &ProjectConfig,
    global_tasks_config: &InheritedTasksConfig,
) -> FileGroupsMap {
    let mut file_groups = FxHashMap::<String, FileGroup>::default();

    debug!(target: log_target, "Creating file groups");

    // Add global file groups first
    for (group_id, files) in &global_tasks_config.file_groups {
        file_groups.insert(
            group_id.to_owned(),
            FileGroup::new(group_id, files.to_owned()),
        );
    }

    // Override global configs with local
    for (group_id, files) in &config.file_groups {
        if file_groups.contains_key(group_id) {
            debug!(
                target: log_target,
                "Merging file group {} with global config",
                color::id(group_id)
            );

            // Group already exists, so merge with it
            file_groups
                .get_mut(group_id)
                .unwrap()
                .merge(files.to_owned());
        } else {
            // Insert a group
            file_groups.insert(group_id.clone(), FileGroup::new(group_id, files.to_owned()));
        }
    }

    file_groups
}

fn create_dependencies_from_config(
    log_target: &str,
    config: &ProjectConfig,
) -> ProjectDependenciesMap {
    let mut deps = FxHashMap::default();

    debug!(target: log_target, "Creating dependencies");

    for dep_cfg in &config.depends_on {
        match dep_cfg {
            ProjectDependsOn::String(id) => {
                deps.insert(
                    id.clone(),
                    ProjectDependency {
                        id: id.clone(),
                        ..ProjectDependency::default()
                    },
                );
            }
            ProjectDependsOn::Object(cfg) => {
                deps.insert(cfg.id.clone(), ProjectDependency::from_config(cfg));
            }
        }
    }

    deps
}

fn create_tasks_from_config(
    log_target: &str,
    project_id: &str,
    project_config: &ProjectConfig,
    global_tasks_config: &InheritedTasksConfig,
) -> Result<TasksMap, ProjectError> {
    let mut tasks = BTreeMap::<String, Task>::new();

    debug!(target: log_target, "Creating tasks");

    // Gather inheritance configs
    let mut include_all = true;
    let mut include: FxHashSet<TaskID> = FxHashSet::default();
    let mut exclude: FxHashSet<TaskID> = FxHashSet::default();
    let mut rename: FxHashMap<TaskID, TaskID> = FxHashMap::default();

    if let Some(rename_config) = &project_config.workspace.inherited_tasks.rename {
        rename.extend(rename_config.clone());
    }

    if let Some(include_config) = &project_config.workspace.inherited_tasks.include {
        include_all = false;
        include.extend(include_config.clone());
    }

    if let Some(exclude_config) = &project_config.workspace.inherited_tasks.exclude {
        exclude.extend(exclude_config.clone());
    }

    // Add global tasks first while taking inheritance config into account
    for (task_id, task_config) in &global_tasks_config.tasks {
        // None = Include all
        // [] = Include none
        // ["a"] = Include "a"
        if !include_all {
            if include.is_empty() {
                trace!(
                    target: log_target,
                    "Not inheriting global tasks, empty `include` set"
                );

                break;
            } else if !include.contains(task_id) {
                trace!(
                    target: log_target,
                    "Not inheriting global task {}, not explicitly included",
                    color::id(task_id)
                );

                continue;
            }
        }

        // None, [] = Exclude none
        // ["a"] = Exclude "a"
        if !exclude.is_empty() && exclude.contains(task_id) {
            trace!(
                target: log_target,
                "Not inheriting global task {}, explicitly excluded",
                color::id(task_id)
            );

            continue;
        }

        let task_name = if rename.contains_key(task_id) {
            let renamed_task_id = rename.get(task_id).unwrap();

            trace!(
                target: log_target,
                "Renaming global task {} to {}",
                color::id(task_id),
                color::id(renamed_task_id)
            );

            renamed_task_id
        } else {
            trace!(
                target: log_target,
                "Inheriting global task {}",
                color::id(task_id)
            );

            task_id
        };

        tasks.insert(
            task_name.to_owned(),
            Task::from_config(Target::new(project_id, task_name)?, task_config)?,
        );
    }

    // Add local tasks second
    for (task_id, task_config) in &project_config.tasks {
        if let Some(existing_task) = tasks.get_mut(task_id) {
            debug!(
                target: log_target,
                "Merging task {} with global config",
                color::id(task_id)
            );

            // Task already exists, so merge with it
            existing_task.merge(task_config)?;
        } else {
            // Insert a new task
            tasks.insert(
                task_id.clone(),
                Task::from_config(Target::new(project_id, task_id)?, task_config)?,
            );
        }
    }

    Ok(tasks)
}

#[derive(Clone, Debug, Default, Deserialize, Display, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectDependencySource {
    #[default]
    #[strum(serialize = "explicit")]
    Explicit,

    #[strum(serialize = "implicit")]
    Implicit,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectDependency {
    pub id: ProjectID,
    pub scope: DependencyScope,
    pub source: ProjectDependencySource,
    pub via: Option<String>,
}

impl ProjectDependency {
    pub fn from_config(config: &DependencyConfig) -> ProjectDependency {
        ProjectDependency {
            id: config.id.clone(),
            scope: config.scope.clone(),
            via: config.via.clone(),
            ..ProjectDependency::default()
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Project {
    /// Unique aliases of the project, alongside its official ID.
    /// This is typically reserved for language specific semantics, like `name` from `package.json`.
    pub aliases: Vec<String>,

    /// Project configuration loaded from "moon.yml", if it exists.
    pub config: ProjectConfig,

    /// List of other projects this project depends on.
    pub dependencies: ProjectDependenciesMap,

    /// File groups specific to the project. Inherits all file groups from the global config.
    pub file_groups: FileGroupsMap,

    /// Unique ID for the project. Is the LHS of the `projects` setting.
    pub id: ProjectID,

    /// Task configuration that was inherited from the global scope.
    pub inherited_config: InheritedTasksConfig,

    /// Primary programming language of the project.
    pub language: ProjectLanguage,

    /// Logging target label.
    #[serde(skip)]
    pub log_target: String,

    /// Absolute path to the project's root folder.
    pub root: PathBuf,

    /// Relative path of the project from the workspace root. Is the RHS of the `projects` setting.
    pub source: FilePath,

    /// Tasks specific to the project. Inherits all tasks from the global config.
    pub tasks: TasksMap,

    /// The type of project.
    #[serde(rename = "type")]
    pub type_of: ProjectType,
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.config == other.config
            && self.file_groups == other.file_groups
            && self.id == other.id
            && self.root == other.root
            && self.source == other.source
            && self.tasks == other.tasks
    }
}

impl Logable for Project {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }
}

impl Project {
    pub fn new<F>(
        id: &str,
        source: &str,
        workspace_root: &Path,
        inherited_tasks: &InheritedTasksManager,
        detect_language: F,
    ) -> Result<Project, ProjectError>
    where
        F: FnOnce(&Path) -> ProjectLanguage,
    {
        let log_target = format!("moon:project:{id}");

        // For the root-level project, the "." dot actually causes
        // a ton of unwanted issues, so just use workspace root directly.
        let root = if source.is_empty() || source == "." {
            workspace_root.to_owned()
        } else {
            workspace_root.join(path::normalize_separators(source))
        };

        debug!(
            target: &log_target,
            "Loading project from {} (id = {}, path = {})",
            color::path(&root),
            color::id(id),
            color::file(source),
        );

        if !root.exists() {
            return Err(ProjectError::MissingProjectAtSource(String::from(source)));
        }

        let config = load_project_config(&log_target, &root, source)?;
        let language = if matches!(config.language, ProjectLanguage::Unknown) {
            detect_language(&root)
        } else {
            config.language
        };

        let global_tasks = inherited_tasks.get_inherited_config(
            config.platform.unwrap_or_else(|| language.into()),
            language,
            config.type_of,
        );
        let file_groups = create_file_groups_from_config(&log_target, &config, &global_tasks);
        let dependencies = create_dependencies_from_config(&log_target, &config);
        let tasks = create_tasks_from_config(&log_target, id, &config, &global_tasks)?;

        Ok(Project {
            aliases: vec![],
            dependencies,
            file_groups,
            id: id.to_owned(),
            language,
            log_target,
            root,
            source: source.to_owned(),
            tasks,
            type_of: config.type_of,
            inherited_config: global_tasks,
            config,
        })
    }

    /// Return a list of project IDs this project depends on.
    pub fn get_dependency_ids(&self) -> Vec<&ProjectID> {
        self.dependencies.keys().collect::<Vec<_>>()
    }

    /// Return a task with the defined ID.
    pub fn get_task(&self, task_id: &str) -> Result<&Task, ProjectError> {
        self.tasks
            .get(task_id)
            .ok_or_else(|| ProjectError::UnconfiguredTask(task_id.to_owned(), self.id.to_owned()))
    }

    /// Return true if this project is affected based on touched files.
    /// Since the project is a folder, we check if a file starts with the root.
    pub fn is_affected(&self, touched_files: &TouchedFilePaths) -> bool {
        for file in touched_files {
            if file.starts_with(&self.root) {
                return true;
            }
        }

        false
    }
}
