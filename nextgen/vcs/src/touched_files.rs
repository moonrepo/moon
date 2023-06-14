use relative_path::RelativePathBuf;
use rustc_hash::FxHashSet;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct TouchedFiles {
    pub added: FxHashSet<RelativePathBuf>,
    pub deleted: FxHashSet<RelativePathBuf>,
    pub modified: FxHashSet<RelativePathBuf>,
    pub untracked: FxHashSet<RelativePathBuf>,

    // Will contain files from the previous fields
    pub staged: FxHashSet<RelativePathBuf>,
    pub unstaged: FxHashSet<RelativePathBuf>,
}

impl TouchedFiles {
    pub fn all(&self) -> Vec<&RelativePathBuf> {
        let mut files = vec![];
        files.extend(&self.added);
        files.extend(&self.deleted);
        files.extend(&self.modified);
        files.extend(&self.untracked);
        files.extend(&self.staged);
        files.extend(&self.unstaged);
        files
    }
}
