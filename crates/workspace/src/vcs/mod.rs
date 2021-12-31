mod git;

use crate::errors::VcsError;
use async_trait::async_trait;
use std::collections::HashSet;

pub type VcsResult<T> = Result<T, VcsError>;

// X          Y     Meaning
// -------------------------------------------------
//          [AMD]   not updated
// M        [ MD]   updated in index
// A        [ MD]   added to index
// D                deleted from index
// R        [ MD]   renamed in index
// C        [ MD]   copied in index
// [MARC]           index and work tree matches
// [ MARC]     M    work tree changed since index
// [ MARC]     D    deleted in work tree
// [ D]        R    renamed in work tree
// [ D]        C    copied in work tree
// -------------------------------------------------
// D           D    unmerged, both deleted
// A           U    unmerged, added by us
// U           D    unmerged, deleted by them
// U           A    unmerged, added by them
// D           U    unmerged, deleted by us
// A           A    unmerged, both added
// U           U    unmerged, both modified
// -------------------------------------------------
// ?           ?    untracked
// !           !    ignored

pub struct TouchedFiles {
    added: HashSet<String>,     // A, C
    deleted: HashSet<String>,   // D
    modified: HashSet<String>,  // M, R
    untracked: HashSet<String>, // ??

    // Will contain files from the previous fields
    staged: HashSet<String>,
    unstaged: HashSet<String>,
}

#[async_trait]
pub trait Vcs {
    async fn get_local_branch(&self) -> VcsResult<String>;
    async fn get_local_hash(&self) -> VcsResult<String>;
    async fn get_origin_branch(&self) -> VcsResult<String>;
    async fn get_origin_hash(&self) -> VcsResult<String>;
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles>;
    async fn run_command(&self, args: Vec<&str>) -> VcsResult<String>;
}
