use crate::errors::ProjectError;
use moon_config::{
    format_error_line, format_figment_errors, ConfigError, DependencyConfig, FilePath,
    GlobalProjectConfig, PlatformType, ProjectConfig, ProjectDependsOn, ProjectID, TaskConfig,
    TaskID,
};
use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_logger::{color, debug, trace, Logable};
use moon_task::{FileGroup, ResolverData, Target, Task, TokenResolver, TouchedFilePaths};
use moon_utils::path;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

type FileGroupsMap = HashMap<String, FileGroup>;

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

    Ok(ProjectConfig::new(project_root))
}

fn create_file_groups_from_config(
    log_target: &str,
    config: &ProjectConfig,
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
) -> Vec<DependencyConfig> {
    let mut deps = vec![];

    debug!(target: log_target, "Creating dependencies");

    for dep_cfg in &config.depends_on {
        match dep_cfg {
            ProjectDependsOn::String(id) => {
                deps.push(DependencyConfig::new(id));
            }
            ProjectDependsOn::Object(cfg) => {
                deps.push(cfg.clone());
            }
        }
    }

    deps
}

fn create_tasks_from_config(
    log_target: &str,
    project_id: &str,
    project_config: &ProjectConfig,
    global_config: &GlobalProjectConfig,
) -> Result<TasksMap, ProjectError> {
    let mut tasks = BTreeMap::<String, Task>::new();

    debug!(target: log_target, "Creating tasks");

    // Gather inheritance configs
    let mut include_all = true;
    let mut include: HashSet<TaskID> = HashSet::new();
    let mut exclude: HashSet<TaskID> = HashSet::new();
    let mut rename: HashMap<TaskID, TaskID> = HashMap::new();

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
            Task::from_config(Target::format(project_id, task_name)?, task_config)?,
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
                Task::from_config(Target::format(project_id, task_id)?, task_config)?,
            );
        }
    }

    Ok(tasks)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Unique alias of the project, alongside its official ID.
    /// This is typically reserved for language specific semantics, like `name` from `package.json`.
    pub alias: Option<String>,

    /// Project configuration loaded from "moon.yml", if it exists.
    pub config: ProjectConfig,

    /// List of other projects this project depends on.
    pub dependencies: Vec<DependencyConfig>,

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
        let dependencies = create_dependencies_from_config(&log_target, &config);
        let tasks = create_tasks_from_config(&log_target, id, &config, global_config)?;

        Ok(Project {
            alias: None,
            config,
            dependencies,
            file_groups,
            id: String::from(id),
            log_target,
            root,
            source: String::from(source),
            tasks,
        })
    }

    // Expand deps, args, inputs, and outputs after all tasks have been created.
    pub fn expand_tasks(
        &mut self,
        workspace_root: &Path,
        implicit_inputs: &[String],
    ) -> Result<(), ProjectError> {
        let resolver_data =
            ResolverData::new(&self.file_groups, workspace_root, &self.root, &self.config);

        for task in self.tasks.values_mut() {
            if matches!(task.platform, PlatformType::Unknown) {
                task.platform = TaskConfig::detect_platform(&self.config, &task.command);
            }

            // Inherit implicit inputs before resolving
            task.inputs.extend(implicit_inputs.iter().cloned());

            // Resolve in order!
            task.expand_env(&resolver_data)?;
            task.expand_deps(&self.id, &self.dependencies)?;
            task.expand_inputs(TokenResolver::for_inputs(&resolver_data))?;
            task.expand_outputs(TokenResolver::for_outputs(&resolver_data))?;

            // Must be last as it references inputs/outputs
            task.expand_args(TokenResolver::for_args(&resolver_data))?;

            // Finalize!
            task.determine_type();
        }

        Ok(())
    }

    /// Return a list of project IDs this project depends on.
    pub fn get_dependency_ids(&self) -> Vec<ProjectID> {
        let mut depends_on = self
            .dependencies
            .iter()
            .map(|d| d.id.clone())
            .collect::<Vec<String>>();

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
