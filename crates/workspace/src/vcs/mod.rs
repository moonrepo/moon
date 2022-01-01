mod git;
mod svn;

use crate::errors::VcsError;
use async_trait::async_trait;
use git::Git;
use moon_config::{VcsManager as VM, WorkspaceConfig};
use moon_logger::{color, debug};
use std::collections::HashSet;
use svn::Svn;

pub type VcsResult<T> = Result<T, VcsError>;

#[allow(dead_code)]
pub struct TouchedFiles {
    added: HashSet<String>,
    deleted: HashSet<String>,
    modified: HashSet<String>,
    untracked: HashSet<String>,

    // Will contain files from the previous fields
    staged: HashSet<String>,
    unstaged: HashSet<String>,
    all: HashSet<String>,
}

#[async_trait]
pub trait Vcs {
    async fn get_local_branch(&self) -> VcsResult<String>;
    async fn get_local_branch_hash(&self) -> VcsResult<String>;
    fn get_default_branch(&self) -> &str;
    async fn get_default_branch_hash(&self) -> VcsResult<String>;
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles>;
    async fn run_command(&self, args: Vec<&str>) -> VcsResult<String>;
}

pub struct VcsManager {}

impl VcsManager {
    pub fn load(config: &WorkspaceConfig) -> Box<dyn Vcs> {
        let vcs_config = config.vcs.as_ref().unwrap();
        let manager = vcs_config.manager.as_ref().unwrap();
        let default_branch = vcs_config.default_branch.as_ref().unwrap().as_str();

        debug!(
            target: "moon:workspace",
            "Using {} version control system",
            color::symbol(&format!("{:?}", manager).to_lowercase())
        );

        match manager {
            VM::Svn => Box::new(Svn::new(default_branch)),
            _ => Box::new(Git::new(default_branch)),
        }
    }
}
