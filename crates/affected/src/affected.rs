use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_task::Target;
use rustc_hash::{FxHashMap, FxHashSet};

pub enum AffectedBy {
    AlwaysAffected,
    DownstreamProject(Id),
    DownstreamTask(Target),
    EnvironmentVariable(String),
    TouchedFile(WorkspaceRelativePathBuf),
    UpstreamProject(Id),
    UpstreamTask(Target),
}

// Dependents
#[derive(Clone, Copy, Default, PartialEq)]
pub enum DownstreamScope {
    #[default]
    None,
    Direct,
    Deep,
}

// Dependencies
#[derive(Clone, Copy, Default, PartialEq)]
pub enum UpstreamScope {
    None,
    Direct,
    #[default]
    Deep,
}

#[derive(Debug, Default, PartialEq)]
pub struct AffectedProjectState {
    pub by_dependencies: FxHashSet<Id>,
    pub by_dependents: FxHashSet<Id>,
    pub by_files: FxHashSet<WorkspaceRelativePathBuf>,
}

impl AffectedProjectState {
    pub fn from(list: Vec<AffectedBy>) -> Self {
        let mut state = Self::default();

        for by in list {
            match by {
                AffectedBy::DownstreamProject(id) => {
                    state.by_dependents.insert(id);
                }
                AffectedBy::TouchedFile(file) => {
                    state.by_files.insert(file);
                }
                AffectedBy::UpstreamProject(id) => {
                    state.by_dependencies.insert(id);
                }
                _ => {}
            };
        }

        state
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct AffectedTaskState {
    pub by_dependencies: FxHashSet<Target>,
    pub by_dependents: FxHashSet<Target>,
    pub by_env: FxHashSet<String>,
    pub by_files: FxHashSet<WorkspaceRelativePathBuf>,
}

impl AffectedTaskState {
    pub fn from(list: Vec<AffectedBy>) -> Self {
        let mut state = Self::default();

        for by in list {
            match by {
                AffectedBy::DownstreamTask(target) => {
                    state.by_dependents.insert(target);
                }
                AffectedBy::EnvironmentVariable(name) => {
                    state.by_env.insert(name);
                }
                AffectedBy::TouchedFile(file) => {
                    state.by_files.insert(file);
                }
                AffectedBy::UpstreamTask(target) => {
                    state.by_dependencies.insert(target);
                }
                _ => {}
            };
        }

        state
    }
}

#[derive(Debug, Default)]
pub struct Affected {
    pub projects: FxHashMap<Id, AffectedProjectState>,
    pub tasks: FxHashMap<Target, AffectedTaskState>,
}

impl Affected {
    pub fn is_project_affected(&self, id: &Id) -> bool {
        self.projects.get(id).is_some()
    }

    pub fn is_task_affected(&self, target: &Target) -> bool {
        self.tasks.get(target).is_some()
    }
}
