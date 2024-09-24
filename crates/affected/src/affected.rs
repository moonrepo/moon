use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use rustc_hash::{FxHashMap, FxHashSet};

pub enum AffectedBy {
    DownstreamProject(Id),
    TouchedFile(WorkspaceRelativePathBuf),
    UpstreamProject(Id),
}

// Dependents
#[derive(PartialEq)]
pub enum DownstreamScope {
    None,
    Direct,
    Deep,
}

// Dependencies
#[derive(PartialEq)]
pub enum UpstreamScope {
    Direct,
    Deep,
}

#[derive(Default)]
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
            };
        }

        state
    }
}

#[derive(Default)]
pub struct Affected {
    pub projects: FxHashMap<Id, AffectedProjectState>,
}
