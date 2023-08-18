use crate::project_error::ProjectError;
use moon_common::{cacheable, path::WorkspaceRelativePathBuf, Id};
use moon_config::{
    DependencyConfig, InheritedTasksResult, LanguageType, PlatformType, ProjectConfig, ProjectType,
};
use moon_file_group::FileGroup;
use moon_query::{Condition, Criteria, Field, LogicalOperator, Queryable};
use moon_task::Task;
use rustc_hash::{FxHashMap, FxHashSet};
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

        /// Task configuration that was inherited from ".moon/tasks".
        pub inherited: Option<InheritedTasksResult>,

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

impl Project {
    /// Return a cache directory for this project, relative from the cache root.
    pub fn get_cache_dir(&self) -> PathBuf {
        PathBuf::from(self.id.as_str())
    }

    /// Return a list of project IDs this project depends on.
    pub fn get_dependency_ids(&self) -> Vec<&Id> {
        self.dependencies.keys().collect::<Vec<_>>()
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

    /// Return true if this project is affected based on touched files.
    /// Since the project is a folder, we check if a file starts with the root.
    pub fn is_affected(&self, touched_files: &FxHashSet<WorkspaceRelativePathBuf>) -> bool {
        touched_files
            .iter()
            .any(|file| file.starts_with(&self.source))
    }
}

impl Queryable for Project {
    /// Return true if this project matches the given query criteria.
    fn matches_criteria(&self, query: &Criteria) -> miette::Result<bool> {
        let match_all = matches!(query.op, LogicalOperator::And);
        let mut matched_any = false;

        for condition in &query.conditions {
            let matches = match condition {
                Condition::Field { field, .. } => {
                    let result = match field {
                        Field::Language(langs) => condition.matches_enum(langs, &self.language),
                        Field::Project(ids) => {
                            if condition.matches(ids, &self.id)? {
                                Ok(true)
                            } else if let Some(alias) = &self.alias {
                                condition.matches(ids, alias)
                            } else {
                                Ok(false)
                            }
                        }
                        Field::ProjectAlias(aliases) => {
                            if let Some(alias) = &self.alias {
                                condition.matches(aliases, alias)
                            } else {
                                Ok(false)
                            }
                        }
                        Field::ProjectName(ids) => condition.matches(ids, &self.id),
                        Field::ProjectSource(sources) => {
                            condition.matches(sources, &self.source.to_string())
                        }
                        Field::ProjectType(types) => condition.matches_enum(types, &self.type_of),
                        Field::Tag(tags) => condition.matches_list(
                            tags,
                            &self
                                .config
                                .tags
                                .iter()
                                .map(|t| t.to_string())
                                .collect::<Vec<_>>(),
                        ),
                        Field::Task(ids) => Ok(self
                            .tasks
                            .values()
                            .any(|task| condition.matches(ids, &task.id).unwrap_or_default())),
                        Field::TaskPlatform(platforms) => Ok(self.tasks.values().any(|task| {
                            condition
                                .matches_enum(platforms, &task.platform)
                                .unwrap_or_default()
                        })),
                        Field::TaskType(types) => Ok(self.tasks.values().any(|task| {
                            condition
                                .matches_enum(types, &task.type_of)
                                .unwrap_or_default()
                        })),
                    };

                    result?
                }
                Condition::Criteria { criteria } => self.matches_criteria(criteria)?,
            };

            if matches {
                matched_any = true;

                if match_all {
                    continue;
                } else {
                    break;
                }
            } else if match_all {
                return Ok(false);
            }
        }

        // No matches using the OR condition
        if !matched_any {
            return Ok(false);
        }

        Ok(true)
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
            && self.tasks == other.tasks
            && self.type_of == other.type_of
    }
}
