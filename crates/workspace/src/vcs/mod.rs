mod git;
mod svn;

use crate::errors::VcsError;
use async_trait::async_trait;
use git::Git;
use moon_config::{VcsConfig, VcsManager as VM};
use moon_logger::{color, debug};
use std::collections::HashSet;
use svn::Svn;

pub type VcsResult<T> = Result<T, VcsError>;

pub struct TouchedFiles {
    added: HashSet<String>,
    deleted: HashSet<String>,
    modified: HashSet<String>,
    untracked: HashSet<String>,

    // Will contain files from the previous fields
    staged: HashSet<String>,
    unstaged: HashSet<String>,
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
    pub fn load(config: &Option<VcsConfig>) -> Box<dyn Vcs> {
        let vcs_config = match config.as_ref() {
            Some(cfg) => cfg.clone(),
            None => VcsConfig::default(),
        };

        debug!(
            target: "moon:workspace",
            "Using {} version control system",
            color::symbol("git")
        );

        let branch = "origin/master";

        match vcs_config.manager {
            Some(VM::Svn) => Box::new(Svn::new(branch)),
            _ => Box::new(Git::new(branch)),
        }
    }
}
