use crate::errors::ProjectError;
use crate::file_group::FileGroup;
use crate::target::Target;
use crate::task::Task;
use crate::token::{TokenResolver, TokenSharedData};
use moon_config::constants::CONFIG_PROJECT_FILENAME;
use moon_config::{
    format_figment_errors, FilePath, GlobalProjectConfig, ProjectConfig, ProjectID, TaskID,
};
use moon_logger::{color, debug, trace, Logable};
use moon_utils::path;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub type FileGroupsMap = HashMap<String, FileGroup>;

pub type ProjectsMap = HashMap<ProjectID, Project>;

pub type TasksMap = HashMap<TaskID, Task>;

// project.yml
fn load_project_config(
    log_target: &str,
    project_root: &Path,
    project_source: &str,
) -> Result<Option<ProjectConfig>, ProjectError> {
    let config_path = project_root.join(CONFIG_PROJECT_FILENAME);

    trace!(
        target: log_target,
        "Attempting to find {} in {}",
        color::file(CONFIG_PROJECT_FILENAME),
        color::path(project_root),
    );

    if config_path.exists() {
        return match ProjectConfig::load(&config_path) {
            Ok(cfg) => Ok(Some(cfg)),
            Err(errors) => Err(ProjectError::InvalidConfigFile(
                String::from(project_source),
                format_figment_errors(errors),
            )),
        };
    }

    Ok(None)
}

fn create_file_groups_from_config(
    log_target: &str,
    config: &Option<ProjectConfig>,
    global_config: &GlobalProjectConfig,
) -> FileGroupsMap {
    let mut file_groups = HashMap::<String, FileGroup>::new();

    debug!(target: log_target, "Creating file groups");

    // Add global file groups first
    for (group_id, files) in &global_config.file_groups {
        file_groups.insert(
            group_id.to_owned(),
            FileGroup::new(group_id, files.to_owned()),
        );
    }

    // Override global configs with local
    if let Some(local_config) = config {
        for (group_id, files) in &local_config.file_groups {
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
    }

    file_groups
}

fn create_tasks_from_config(
    log_target: &str,
    config: &Option<ProjectConfig>,
    global_config: &GlobalProjectConfig,
    workspace_root: &Path,
    project_root: &Path,
    project_id: &str,
    file_groups: &FileGroupsMap,
) -> Result<TasksMap, ProjectError> {
    let mut tasks = HashMap::<String, Task>::new();
    let mut depends_on = vec![];

    debug!(target: log_target, "Creating tasks");

    // Gather inheritance configs
    let mut include_all = true;
    let mut include: HashSet<TaskID> = HashSet::new();
    let mut exclude: HashSet<TaskID> = HashSet::new();
    let mut rename: HashMap<TaskID, TaskID> = HashMap::new();

    if let Some(local_config) = config {
        depends_on.extend(local_config.depends_on.clone());
        rename = local_config.workspace.inherited_tasks.rename.clone();

        if let Some(include_config) = &local_config.workspace.inherited_tasks.include {
            include_all = false;
            include.extend(include_config.clone());
        }

        if let Some(exclude_config) = &local_config.workspace.inherited_tasks.exclude {
            exclude.extend(exclude_config.clone());
        }
    }

    // Add global tasks first while taking inheritance config into account
    for (task_id, task_config) in &global_config.tasks {
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
            task_id
        };

        tasks.insert(
            task_name.to_owned(),
            Task::from_config(Target::format(project_id, task_name)?, task_config),
        );
    }

    // Add local tasks second
    if let Some(local_config) = config {
        for (task_id, task_config) in &local_config.tasks {
            if tasks.contains_key(task_id) {
                debug!(
                    target: log_target,
                    "Merging task {} with global config",
                    color::id(task_id)
                );

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

    // Expand deps, args, inputs, and outputs after all tasks have been created
    for task in tasks.values_mut() {
        let data = TokenSharedData::new(file_groups, workspace_root, project_root, config.as_ref());

        task.expand_deps(project_id, &depends_on)?;
        task.expand_inputs(TokenResolver::for_inputs(&data))?;
        task.expand_outputs(TokenResolver::for_outputs(&data))?;

        // Must be last as it references inputs/outputs
        task.expand_args(TokenResolver::for_args(&data))?;
    }

    Ok(tasks)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Project configuration loaded from "project.yml", if it exists.
    pub config: Option<ProjectConfig>,

    /// File groups specific to the project. Inherits all file groups from the global config.
    pub file_groups: FileGroupsMap,

    /// Unique ID for the project. Is the LHS of the `projects` setting.
    pub id: ProjectID,

    /// Logging target label.
    #[serde(skip)]
    pub log_target: String,

    /// Absolute path to the project's root folder.
    pub root: PathBuf,

    /// Relative path of the project from the workspace root. Is the RHS of the `projects` setting.
    pub source: FilePath,

    /// Tasks specific to the project. Inherits all tasks from the global config.
    pub tasks: TasksMap,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            config: None,
            file_groups: HashMap::new(),
            id: String::new(),
            log_target: String::new(),
            root: PathBuf::new(),
            source: String::new(),
            tasks: HashMap::new(),
        }
    }
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
    pub fn new(
        id: &str,
        source: &str,
        workspace_root: &Path,
        global_config: &GlobalProjectConfig,
    ) -> Result<Project, ProjectError> {
        let root = workspace_root.join(path::normalize_separators(source));
        let log_target = format!("moon:project:{}", id);

        debug!(
            target: &log_target,
            "Loading project from {} (id = {}, path = {})",
            color::path(&root),
            color::id(id),
            color::file(source),
        );

        if !root.exists() {
            return Err(ProjectError::MissingProject(String::from(source)));
        }

        let config = load_project_config(&log_target, &root, source)?;
        let file_groups = create_file_groups_from_config(&log_target, &config, global_config);
        let tasks = create_tasks_from_config(
            &log_target,
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
            log_target,
            root,
            source: String::from(source),
            tasks,
        })
    }

    /// Return a list of project IDs this project depends on.
    pub fn get_dependencies(&self) -> Vec<ProjectID> {
        let mut depends_on = vec![];

        if let Some(config) = &self.config {
            depends_on.extend_from_slice(&config.depends_on);
        }

        depends_on.sort();

        depends_on
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
}
