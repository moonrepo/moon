use crate::errors::VcsError;
use async_trait::async_trait;
use moon_process::Command;
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;

pub type VcsResult<T> = Result<T, VcsError>;

#[allow(dead_code)]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TouchedFiles {
    pub added: FxHashSet<String>,
    pub deleted: FxHashSet<String>,
    pub modified: FxHashSet<String>,
    pub untracked: FxHashSet<String>,

    // Will contain files from the previous fields
    pub staged: FxHashSet<String>,
    pub unstaged: FxHashSet<String>,
    pub all: FxHashSet<String>,
}

#[async_trait]
pub trait Vcs {
    /// Create a process command for the underlying vcs binary.
    fn create_command(&self, args: Vec<&str>) -> Command;

    /// Get the local checkout branch name.
    async fn get_local_branch(&self) -> VcsResult<String>;

    /// Get the revision hash/number of the local branch's HEAD.
    async fn get_local_branch_revision(&self) -> VcsResult<String>;

    /// Get the remote checkout default name. Typically master/main on git, and trunk on svn.
    fn get_default_branch(&self) -> &str;

    /// Get the revision hash/number of the default branch's HEAD.
    async fn get_default_branch_revision(&self) -> VcsResult<String>;

    /// Get a map of hashes for the provided files.
    /// Files returned are relative from the repository root.
    async fn get_file_hashes(
        &self,
        files: &[String],
        allow_ignored: bool,
        batch_size: Option<u16>,
    ) -> VcsResult<BTreeMap<String, String>>;

    /// Get a list of all files in the provided directory, recursing through all sub-directories.
    /// Files returned are relative from the repository root.
    async fn get_file_tree(&self, dir: &str) -> VcsResult<Vec<String>>;

    /// Return the repository slug ("moonrepo/moon") of the current checkout.
    async fn get_repository_slug(&self) -> VcsResult<String>;

    /// Determine touched files from the local index / working tree.
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles>;

    /// Determine touched files between a revision and it's self -1 revision.
    async fn get_touched_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> VcsResult<TouchedFiles>;

    /// Determine touched files between 2 revisions.
    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> VcsResult<TouchedFiles>;

    /// Return true if the provided branch matches the default branch.
    fn is_default_branch(&self, branch: &str) -> bool;

    /// Return true if the repo is currently VCS enabled.
    fn is_enabled(&self) -> bool;

    /// Return true if the provided file path has been ignored.
    fn is_ignored(&self, file: &str) -> bool;
}
