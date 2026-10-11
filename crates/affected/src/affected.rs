use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_task::Target;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum AffectedBy {
    AlreadyMarked,
    AlwaysAffected,
    ChangedFile(WorkspaceRelativePathBuf),
    EnvironmentVariable(String),
    /// A project is affected because one of its own tasks is.
    Task(Target),
    /// A project is affected because a project it depends on is.
    DependencyProject(Id),
    /// A task is affected because a task it depends on is.
    DependencyTask(Target),
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AffectedProjectState {
    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub files: FxHashSet<WorkspaceRelativePathBuf>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub tasks: FxHashSet<Target>,

    /// Projects this project depends on that are affected,
    /// and that it was reached through.
    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub dependencies: FxHashSet<Id>,

    pub other: bool,
}

impl AffectedProjectState {
    pub fn from(list: FxHashSet<AffectedBy>) -> Self {
        let mut state = Self::default();

        for by in list {
            match by {
                AffectedBy::ChangedFile(file) => {
                    state.files.insert(file);
                }
                AffectedBy::DependencyProject(id) => {
                    state.dependencies.insert(id);
                }
                AffectedBy::Task(target) => {
                    state.tasks.insert(target);
                }
                _ => {
                    state.other = true;
                }
            };
        }

        state
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AffectedTaskState {
    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub env: FxHashSet<String>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub files: FxHashSet<WorkspaceRelativePathBuf>,

    /// Tasks this task depends on that are affected,
    /// and that it was reached through.
    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub dependencies: FxHashSet<Target>,

    pub other: bool,
}

impl AffectedTaskState {
    pub fn from(list: FxHashSet<AffectedBy>) -> Self {
        let mut state = Self::default();

        for by in list {
            match by {
                AffectedBy::EnvironmentVariable(name) => {
                    state.env.insert(name);
                }
                AffectedBy::ChangedFile(file) => {
                    state.files.insert(file);
                }
                AffectedBy::DependencyTask(target) => {
                    state.dependencies.insert(target);
                }
                _ => {
                    state.other = true;
                }
            };
        }

        state
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Affected {
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub projects: FxHashMap<Id, AffectedProjectState>,

    #[serde(skip)]
    pub should_check: bool,

    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub tasks: FxHashMap<Target, AffectedTaskState>,
}

impl Affected {
    pub fn is_project_affected(&self, id: &Id) -> bool {
        self.should_check && self.projects.contains_key(id)
    }

    pub fn is_task_affected(&self, target: &Target) -> bool {
        self.should_check && self.tasks.contains_key(target)
    }
}
