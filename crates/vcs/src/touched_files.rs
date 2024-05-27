use moon_common::path::WorkspaceRelativePathBuf;
use rustc_hash::FxHashSet;

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
