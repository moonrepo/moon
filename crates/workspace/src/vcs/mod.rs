mod git;
mod svn;

use crate::errors::WorkspaceError;
use async_trait::async_trait;
use git::Git;
use moon_config::{VcsManager as VM, WorkspaceConfig};
use moon_logger::{color, debug};
use std::collections::HashSet;
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
    async fn get_local_branch(&self) -> VcsResult<String>;
    async fn get_local_branch_hash(&self) -> VcsResult<String>;
    fn get_default_branch(&self) -> &str;
    async fn get_default_branch_hash(&self) -> VcsResult<String>;
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles>;
    async fn get_touched_files_against_branch(
        &self,
        branch: &str,
        back_revision_count: u8,
    ) -> VcsResult<TouchedFiles>;

    /// Return true if the provided branch matches the default branch.
    fn is_default_branch(&self, branch: &str) -> bool;

    async fn run_command(&self, args: Vec<&str>, trim: bool) -> VcsResult<String>;
}

pub struct VcsManager {}

impl VcsManager {
    pub fn load(config: &WorkspaceConfig, working_dir: &Path) -> Box<dyn Vcs> {
        let vcs_config = config.vcs.as_ref().unwrap();
        let manager = vcs_config.manager.as_ref().unwrap();
        let default_branch = vcs_config.default_branch.as_ref().unwrap().as_str();

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
