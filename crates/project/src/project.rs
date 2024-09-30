use crate::project_error::ProjectError;
use moon_common::{cacheable, path::WorkspaceRelativePathBuf, Id};
use moon_config::{
    DependencyConfig, InheritedTasksResult, LanguageType, PlatformType, ProjectConfig, ProjectType,
    StackType,
};
use moon_file_group::FileGroup;
use moon_task::Task;
use std::collections::BTreeMap;
use std::path::PathBuf;

cacheable!(
    #[derive(Clone, Debug, Default)]
    pub struct Project {
        /// Unique alias of the project, alongside its official ID.
        /// This is typically for language specific semantics, like `name` from `package.json`.
        pub alias: Option<String>,

        /// Project configuration loaded from "moon.yml", if it exists.
        pub config: ProjectConfig,

        /// List of other projects this project depends on.
        pub dependencies: Vec<DependencyConfig>,

        /// File groups specific to the project. Inherits all file groups from the global config.
        pub file_groups: BTreeMap<Id, FileGroup>,

        /// Unique ID for the project. Is the LHS of the `projects` setting.
        pub id: Id,

        /// Task configuration that was inherited from ".moon/tasks".
        pub inherited: Option<InheritedTasksResult>,

        /// Primary programming language of the project.
        pub language: LanguageType,

        /// Default platform to run tasks against.
        pub platform: PlatformType,

        /// The technology stack of the project.
        pub stack: StackType,

        /// Absolute path to the project's root folder.
        pub root: PathBuf,

        /// Relative path from the workspace root to the project root.
        /// Is the RHS of the `projects` setting.
        pub source: WorkspaceRelativePathBuf,

        /// Tasks specific to the project. Inherits all tasks from the global config.
        pub tasks: BTreeMap<Id, Task>,

        /// The type of project.
        #[serde(rename = "type")]
        pub type_of: ProjectType,
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

    /// Return a task with the defined ID.
    pub fn get_task<I: AsRef<str>>(&self, task_id: I) -> miette::Result<&Task> {
        let task_id = Id::raw(task_id.as_ref());

        let task = self
            .tasks
            .get(&task_id)
            .ok_or_else(|| ProjectError::UnknownTask {
                task_id: task_id.clone(),
                project_id: self.id.clone(),
            })?;

        if !task.is_expanded() {
            return Err(ProjectError::UnexpandedTask {
                task_id,
                project_id: self.id.clone(),
            })?;
        }

        Ok(task)
    }

    /// Return a list of all visible task IDs.
    pub fn get_task_ids(&self) -> miette::Result<Vec<&Id>> {
        Ok(self
            .get_tasks()?
            .iter()
            .map(|task| &task.id)
            .collect::<Vec<_>>())
    }

    /// Return all visible tasks within the project. Does not include internal tasks!
    pub fn get_tasks(&self) -> miette::Result<Vec<&Task>> {
        let mut tasks = vec![];

        for task_id in self.tasks.keys() {
            let task = self.get_task(task_id)?;

            if !task.is_internal() {
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    /// Return true if the root-level project.
    pub fn is_root_level(&self) -> bool {
        self.source.as_str().is_empty() || self.source.as_str() == "."
    }

    /// Return true if the provided locator string (an ID or alias) matches the
    /// current project.
    pub fn matches_locator(&self, locator: &str) -> bool {
        self.id.as_str() == locator || self.alias.as_ref().is_some_and(|alias| alias == locator)
    }
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.alias == other.alias
            && self.file_groups == other.file_groups
            && self.id == other.id
            && self.language == other.language
            && self.root == other.root
            && self.source == other.source
            && self.stack == other.stack
            && self.tasks == other.tasks
            && self.type_of == other.type_of
    }
}
