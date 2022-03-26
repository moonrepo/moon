mod git;
mod svn;

use crate::errors::WorkspaceError;
use async_trait::async_trait;
use git::Git;
use moon_config::{VcsManager as VM, WorkspaceConfig};
use moon_logger::{color, debug};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use svn::Svn;

pub type VcsResult<T> = Result<T, WorkspaceError>;

#[allow(dead_code)]
#[derive(Default)]
pub struct TouchedFiles {
    pub added: HashSet<String>,
    pub deleted: HashSet<String>,
    pub modified: HashSet<String>,
    pub untracked: HashSet<String>,

    // Will contain files from the previous fields
    pub staged: HashSet<String>,
    pub unstaged: HashSet<String>,
    pub all: HashSet<String>,
}

#[async_trait]
pub trait Vcs {
    /// Get the local checkout branch name.
    async fn get_local_branch(&self) -> VcsResult<String>;

    /// Get the revision hash/number of the local branch's HEAD.
    async fn get_local_branch_revision(&self) -> VcsResult<String>;

    /// Get the upstream checkout default name. Typically master/main on git, and trunk on svn.
    fn get_default_branch(&self) -> &str;

    /// Get the revision hash/number of the default branch's HEAD.
    async fn get_default_branch_revision(&self) -> VcsResult<String>;

    async fn get_file_hashes(&self, files: &[String]) -> VcsResult<BTreeMap<String, String>>;

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

    /// Execute the underlying vcs binary.
    async fn run_command(&self, args: Vec<&str>, trim: bool) -> VcsResult<String>;
}

pub struct VcsManager {}

impl VcsManager {
    pub fn load(config: &WorkspaceConfig, working_dir: &Path) -> Box<dyn Vcs + Send + Sync> {
        let vcs_config = &config.vcs;
        let manager = &vcs_config.manager;
        let default_branch = &vcs_config.default_branch;

        debug!(
            target: "moon:workspace",
            "Using {} version control system",
            color::symbol(&format!("{:?}", manager).to_lowercase())
        );

        match manager {
            VM::Svn => Box::new(Svn::new(default_branch, working_dir)),
            _ => Box::new(Git::new(default_branch, working_dir)),
        }
    }
}
