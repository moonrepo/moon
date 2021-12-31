mod git;
mod svn;

use crate::errors::VcsError;
use async_trait::async_trait;
use git::Git;
use moon_logger::{color, debug, trace};
use std::collections::HashSet;
use std::path::Path;
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
    async fn get_local_hash(&self) -> VcsResult<String>;
    async fn get_origin_branch(&self) -> VcsResult<String>;
    async fn get_origin_hash(&self) -> VcsResult<String>;
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles>;
    async fn run_command(&self, args: Vec<&str>) -> VcsResult<String>;
}

pub struct VcsDetector {}

impl VcsDetector {
    pub fn detect(workspace_root: &Path, origin_branch: &str) -> Result<Box<dyn Vcs>, VcsError> {
        debug!(
            target: "moon:workspace:vcs",
            "Detecting version control system, starting from {}",
            color::file_path(workspace_root)
        );

        if find_config_dir(workspace_root, "git") {
            return Ok(Box::new(Git::new(origin_branch)));
        }

        if find_config_dir(workspace_root, "svn") {
            return Ok(Box::new(Svn::new()));
        }

        Err(VcsError::FailedDetection)
    }
}

fn find_config_dir(starting_dir: &Path, vcs: &str) -> bool {
    let vcs_dir_name = format!(".{}", vcs);
    let config_dir = starting_dir.join(vcs_dir_name);

    trace!(
        target: "moon:workspace:vcs",
        "Attempting to find {} config folder {}",
        color::symbol(vcs),
        color::file_path(&config_dir),
    );

    if config_dir.exists() {
        return true;
    }

    let parent_dir = starting_dir.parent();

    match parent_dir {
        Some(dir) => find_config_dir(dir, vcs),
        None => false,
    }
}
