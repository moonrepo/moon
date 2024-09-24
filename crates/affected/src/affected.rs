use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use rustc_hash::{FxHashMap, FxHashSet};

pub enum AffectedBy {
    DownstreamProject(Id),
    TouchedFile(WorkspaceRelativePathBuf),
    UpstreamProject(Id),
}

// Dependents
#[derive(Default, PartialEq)]
pub enum DownstreamScope {
    #[default]
    None,
    Direct,
    Deep,
}

// Dependencies
#[derive(Default, PartialEq)]
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
            };
        }

        state
    }
}

#[derive(Debug, Default)]
pub struct Affected {
    pub projects: FxHashMap<Id, AffectedProjectState>,
}

impl Affected {
    pub fn is_project_affected(&self, id: &Id) -> bool {
        self.projects
            .get(id)
            .map(|state| {
                !state.by_dependencies.is_empty()
                    || !state.by_dependents.is_empty()
                    || !state.by_files.is_empty()
            })
            .unwrap_or(false)
    }
}
