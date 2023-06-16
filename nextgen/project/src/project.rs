use moon_common::{cacheable, path::WorkspaceRelativePathBuf, Id};
use moon_config::{DependencyConfig, LanguageType, PlatformType, ProjectConfig, ProjectType};
use moon_file_group::FileGroup;
use moon_task2::Task;
use rustc_hash::FxHashMap;
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
        pub dependencies: FxHashMap<Id, DependencyConfig>,

        /// File groups specific to the project. Inherits all file groups from the global config.
        pub file_groups: FxHashMap<Id, FileGroup>,

        /// Unique ID for the project. Is the LHS of the `projects` setting.
        pub id: Id,

        // TODO
        /// Task configuration that was inherited from the global scope.
        // pub inherited_config: InheritedTasksConfig,

        /// Primary programming language of the project.
        pub language: LanguageType,

        /// Default platform to run tasks against.
        pub platform: PlatformType,

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

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.alias == other.alias
            && self.file_groups == other.file_groups
            && self.id == other.id
            && self.language == other.language
            && self.root == other.root
            && self.source == other.source
            && self.tasks == other.tasks
            && self.type_of == other.type_of
    }
}
