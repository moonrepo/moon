use moon_common::path::WorkspaceRelativePathBuf;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct TouchedFiles {
    pub added: FxHashSet<WorkspaceRelativePathBuf>,
    pub deleted: FxHashSet<WorkspaceRelativePathBuf>,
    pub modified: FxHashSet<WorkspaceRelativePathBuf>,
    pub untracked: FxHashSet<WorkspaceRelativePathBuf>,

    // Will contain files from the previous fields
    pub staged: FxHashSet<WorkspaceRelativePathBuf>,
    pub unstaged: FxHashSet<WorkspaceRelativePathBuf>,
}

impl TouchedFiles {
    pub fn all(&self) -> FxHashSet<&WorkspaceRelativePathBuf> {
        let mut files = FxHashSet::default();
        files.extend(&self.added);
        files.extend(&self.deleted);
        files.extend(&self.modified);
        files.extend(&self.untracked);
        files.extend(&self.staged);
        files.extend(&self.unstaged);
        files
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TouchedStatus {
    Added,
    #[default]
    All,
    Deleted,
    Modified,
    Staged,
    Unstaged,
    Untracked,
}

impl fmt::Display for TouchedStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                TouchedStatus::Added => "added",
                TouchedStatus::All => "all",
                TouchedStatus::Deleted => "deleted",
                TouchedStatus::Modified => "modified",
                TouchedStatus::Staged => "staged",
                TouchedStatus::Unstaged => "unstaged",
                TouchedStatus::Untracked => "untracked",
            }
        )?;

        Ok(())
    }
}
