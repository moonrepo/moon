use crate::project_error::ProjectError;
use moon_common::{
    Id, cacheable,
    path::{WorkspaceRelativePathBuf, is_root_level_source},
};
use moon_config::{
    DependencyScope, InheritedTasksResult, LanguageType, LayerType, ProjectConfig,
    ProjectDependencyConfig, StackType,
};
use moon_file_group::FileGroup;
use moon_task::{Target, Task};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

cacheable!(
    #[derive(Clone, Debug, Default)]
    #[serde(default)]
    pub struct Project {
        /// Unique alias(es) of the project, alongside its official identifier.
        /// This is typically for language specific semantics, like `name` from `package.json`.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub aliases: Vec<String>,

        /// Project configuration loaded from "moon.*", if it exists.
        pub config: ProjectConfig,

        /// List of other projects this project depends on.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub dependencies: Vec<ProjectDependencyConfig>,

        /// File groups specific to the project. Inherits all file groups from the global config.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub file_groups: BTreeMap<Id, FileGroup>,

        /// Unique ID for the project. Is the LHS of the `projects` setting.
        pub id: Id,

        /// Task configuration that was inherited from ".moon/tasks".
        #[serde(skip_serializing_if = "Option::is_none")]
        pub inherited: Option<InheritedTasksResult>,

        /// Primary programming language of the project.
        pub language: LanguageType,

        /// The type of layer within the stack. Is used for layer constraints.
        pub layer: LayerType,

        /// Absolute path to the project's root folder.
        pub root: PathBuf,

        /// Relative path from the workspace root to the project root.
        pub source: WorkspaceRelativePathBuf,

        /// The technology stack of the project.
        pub stack: StackType,

        /// Tasks specific to the project. Inherits all tasks from the global config.
        /// Note: This map is empty when the project is in the project graph!
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub tasks: BTreeMap<Id, Task>,

        /// List of targets of all tasks configured or inherited for the project.
        /// Includes internal tasks!
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub task_targets: Vec<Target>,

        /// Toolchains derived from the configured language.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub toolchains: Vec<Id>,
    }
);

impl Project {
    /// Return a list of project IDs this project depends on.
    pub fn get_dependency_ids(&self) -> Vec<&Id> {
        self.dependencies
            .iter()
            .map(|dep| &dep.id)
            .collect::<Vec<_>>()
    }

    /// Return a list of all toolchains that are enabled for this project.
    /// Toolchains can be disabled through config.
    pub fn get_enabled_toolchains(&self) -> Vec<&Id> {
        self.toolchains
            .iter()
            .filter(|id| match self.config.toolchains.get_plugin_config(*id) {
                None => true,
                Some(cfg) => cfg.is_enabled(),
            })
            .collect()
    }

    /// Return a list of all task specific toolchains that are enabled for this project.
    /// Toolchains can be disabled through config.
    pub fn get_enabled_toolchains_for_task<'task>(&self, task: &'task Task) -> Vec<&'task Id> {
        task.toolchains
            .iter()
            .filter(|id| match self.config.toolchains.get_plugin_config(*id) {
                None => true,
                Some(cfg) => cfg.is_enabled(),
            })
            .collect()
    }

    /// Return the file group by ID or fail with an error.
    pub fn get_file_group(&self, group_id: impl AsRef<str>) -> Result<&FileGroup, ProjectError> {
        self.file_groups
            .get(group_id.as_ref())
            .ok_or_else(|| ProjectError::UnknownFileGroup {
                group_id: group_id.as_ref().to_string(),
                project_id: self.id.to_string(),
            })
    }

    /// Return the task by ID or fail with an error.
    pub fn get_task(&self, task_id: impl AsRef<str>) -> Result<&Task, ProjectError> {
        self.tasks
            .get(task_id.as_ref())
            .ok_or_else(|| ProjectError::UnknownTask {
                task_id: task_id.as_ref().to_string(),
                project_id: self.id.to_string(),
            })
    }

    /// Return true if the root-level project.
    pub fn is_root_level(&self) -> bool {
        is_root_level_source(&self.source)
    }

    /// Return true if the provided locator string (an ID or alias) matches the
    /// current project.
    pub fn matches_locator(&self, locator: &str) -> bool {
        self.id.as_str() == locator || self.aliases.iter().any(|alias| alias == locator)
    }

    /// Convert the project into a fragment.
    pub fn to_fragment(&self) -> ProjectFragment {
        ProjectFragment {
            aliases: self.aliases.clone(),
            dependency_scope: None,
            id: self.id.clone(),
            source: self.source.to_string(),
            toolchains: self.get_enabled_toolchains().into_iter().cloned().collect(),
        }
    }
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.aliases == other.aliases
            && self.file_groups == other.file_groups
            && self.id == other.id
            && self.language == other.language
            && self.layer == other.layer
            && self.root == other.root
            && self.source == other.source
            && self.stack == other.stack
            && self.tasks == other.tasks
            && self.task_targets == other.task_targets
    }
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

cacheable!(
    /// Fragment of a project including important fields.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct ProjectFragment {
        /// Alias of the project.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub aliases: Vec<String>,

        /// When treated as a dependency for another project,
        /// the scope of that dependency relationship.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dependency_scope: Option<DependencyScope>,

        /// ID of the project.
        pub id: Id,

        /// Workspace relative path to the project root.
        pub source: String,

        /// Toolchains the project belongs to. Does not include
        /// toolchains that have been disabled through config.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub toolchains: Vec<Id>,
    }
);
