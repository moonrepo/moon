use clap::ValueEnum;
use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_task::Target;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum AffectedBy {
    AlreadyMarked,
    AlwaysAffected,
    DownstreamProject(Id),
    DownstreamTask(Target),
    EnvironmentVariable(String),
    Task(Target),
    TouchedFile(WorkspaceRelativePathBuf),
    UpstreamProject(Id),
    UpstreamTask(Target),
}

// Dependents
#[derive(Clone, Copy, Debug, Default, PartialEq, ValueEnum)]
pub enum DownstreamScope {
    #[default]
    None,
    Direct,
    Deep,
}

impl fmt::Display for DownstreamScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::Direct => "direct",
                Self::Deep => "deep",
            }
        )
    }
}

// Dependencies
#[derive(Clone, Copy, Debug, Default, PartialEq, ValueEnum)]
pub enum UpstreamScope {
    #[default]
    None,
    Direct,
    Deep,
}

impl fmt::Display for UpstreamScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::Direct => "direct",
                Self::Deep => "deep",
            }
        )
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AffectedProjectState {
    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub files: FxHashSet<WorkspaceRelativePathBuf>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub tasks: FxHashSet<Target>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub upstream: FxHashSet<Id>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub downstream: FxHashSet<Id>,

    pub other: bool,
}

impl AffectedProjectState {
    pub fn from(list: FxHashSet<AffectedBy>) -> Self {
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

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub upstream: FxHashSet<Target>,

    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub downstream: FxHashSet<Target>,

    pub other: bool,
}

impl AffectedTaskState {
    pub fn from(list: FxHashSet<AffectedBy>) -> Self {
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
