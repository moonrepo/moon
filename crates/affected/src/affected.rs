use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_task::Target;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;

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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DownstreamScope {
    #[default]
    None,
    Direct,
    Deep,
}

// Dependencies
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum UpstreamScope {
    None,
    Direct,
    #[default]
    Deep,
}

#[derive(Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AffectedProjectState {
    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub files: FxHashSet<WorkspaceRelativePathBuf>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub upstream: FxHashSet<Id>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub downstream: FxHashSet<Id>,
}

impl AffectedProjectState {
    pub fn from(list: Vec<AffectedBy>) -> Self {
        let mut state = Self::default();

        for by in list {
            match by {
                AffectedBy::DownstreamProject(id) => {
                    state.downstream.insert(id);
                }
                AffectedBy::TouchedFile(file) => {
                    state.files.insert(file);
                }
                AffectedBy::UpstreamProject(id) => {
                    state.upstream.insert(id);
                }
                _ => {}
            };
        }

        state
    }
}

#[derive(Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AffectedTaskState {
    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub env: FxHashSet<String>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub files: FxHashSet<WorkspaceRelativePathBuf>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub upstream: FxHashSet<Target>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub downstream: FxHashSet<Target>,
}

impl AffectedTaskState {
    pub fn from(list: Vec<AffectedBy>) -> Self {
        let mut state = Self::default();

        for by in list {
            match by {
                AffectedBy::DownstreamTask(target) => {
                    state.downstream.insert(target);
                }
                AffectedBy::EnvironmentVariable(name) => {
                    state.env.insert(name);
                }
                AffectedBy::TouchedFile(file) => {
                    state.files.insert(file);
                }
                AffectedBy::UpstreamTask(target) => {
                    state.upstream.insert(target);
                }
                _ => {}
            };
        }

        state
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Affected {
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub projects: FxHashMap<Id, AffectedProjectState>,

    pub should_check: bool,

    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub tasks: FxHashMap<Target, AffectedTaskState>,
}

impl Affected {
    pub fn is_project_affected(&self, id: &Id) -> bool {
        self.should_check && self.projects.get(id).is_some()
    }

    pub fn is_task_affected(&self, target: &Target) -> bool {
        self.should_check && self.tasks.get(target).is_some()
    }
}
