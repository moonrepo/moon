use moon_common::{
    cacheable,
    path::{is_root_level_source, WorkspaceRelativePathBuf},
    serde::is_wasm_bridge,
    Id,
};
use moon_config::{
    DependencyConfig, InheritedTasksResult, LanguageType, PlatformType, ProjectConfig, ProjectType,
    StackType,
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
        /// Unique alias of the project, alongside its official ID.
        /// This is typically for language specific semantics, like `name` from `package.json`.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub alias: Option<String>,

        /// Project configuration loaded from "moon.*", if it exists.
        #[serde(skip_serializing_if = "is_wasm_bridge")]
        pub config: ProjectConfig,

        /// List of other projects this project depends on.
        pub dependencies: Vec<DependencyConfig>,

        /// File groups specific to the project. Inherits all file groups from the global config.
        pub file_groups: BTreeMap<Id, FileGroup>,

        /// Unique ID for the project. Is the LHS of the `projects` setting.
        pub id: Id,

        /// Task configuration that was inherited from ".moon/tasks".
        #[serde(skip_serializing_if = "is_wasm_bridge")]
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

        /// List of targets of all tasks configured or inherited for the project.
        /// Includes internal tasks!
        pub task_targets: Vec<Target>,

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

    /// Return true if the root-level project.
    pub fn is_root_level(&self) -> bool {
        is_root_level_source(&self.source)
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
            && self.task_targets == other.task_targets
            && self.type_of == other.type_of
    }
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
