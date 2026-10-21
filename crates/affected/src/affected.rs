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
    /// A project is affected because a project it depends on is, which is
    /// the provided distance (in hops) away from a directly affected project.
    DependencyProject {
        id: Id,
        distance: u32,
    },
    /// A task is affected because a task it depends on is, which is the
    /// provided distance (in hops) away from a directly affected task.
    DependencyTask {
        target: Target,
        distance: u32,
    },
}

/// Fold the distance of a dependency mark into the minimum distance.
fn min_distance(current: Option<u32>, distance: u32) -> Option<u32> {
    Some(current.map_or(distance, |current| current.min(distance)))
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

    /// How many hops away from a directly affected project this project is.
    /// 0 when it is directly affected (by a file, or by one of its own tasks),
    /// otherwise the minimum distance over its affected dependencies.
    pub distance: u32,

    pub other: bool,
}

impl AffectedProjectState {
    pub fn from(list: FxHashSet<AffectedBy>) -> Self {
        let mut state = Self::default();
        let mut direct = false;
        let mut distance = None;

        for by in list {
            match by {
                AffectedBy::ChangedFile(file) => {
                    state.files.insert(file);
                    direct = true;
                }
                AffectedBy::DependencyProject {
                    id,
                    distance: dep_distance,
                } => {
                    state.dependencies.insert(id);
                    distance = min_distance(distance, dep_distance);
                }
                AffectedBy::Task(target) => {
                    state.tasks.insert(target);
                    direct = true;
                }
                _ => {
                    state.other = true;
                    direct = true;
                }
            };
        }

        if !direct {
            state.distance = distance.unwrap_or_default();
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

    /// How many hops away from a directly affected task this task is.
    /// 0 when it is directly affected (by a file, an environment variable,
    /// or always), otherwise the minimum distance over its affected dependencies.
    pub distance: u32,

    pub other: bool,
}

impl AffectedTaskState {
    pub fn from(list: FxHashSet<AffectedBy>) -> Self {
        let mut state = Self::default();
        let mut direct = false;
        let mut distance = None;

        for by in list {
            match by {
                AffectedBy::EnvironmentVariable(name) => {
                    state.env.insert(name);
                    direct = true;
                }
                AffectedBy::ChangedFile(file) => {
                    state.files.insert(file);
                    direct = true;
                }
                AffectedBy::DependencyTask {
                    target,
                    distance: dep_distance,
                } => {
                    state.dependencies.insert(target);
                    distance = min_distance(distance, dep_distance);
                }
                _ => {
                    state.other = true;
                    direct = true;
                }
            };
        }

        if !direct {
            state.distance = distance.unwrap_or_default();
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
