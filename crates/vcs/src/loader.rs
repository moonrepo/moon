use crate::errors::VcsError;
use crate::git::Git;
use crate::svn::Svn;
use crate::vcs::Vcs;
use moon_config::{VcsManager, WorkspaceConfig};
use std::path::Path;

pub struct VcsLoader {}

impl VcsLoader {
    pub fn load(
        working_dir: &Path,
        workspace_config: &WorkspaceConfig,
    ) -> Result<Box<dyn Vcs + Send + Sync>, VcsError> {
        let vcs_config = &workspace_config.vcs;
        let manager = &vcs_config.manager;
        let default_branch = &vcs_config.default_branch;

        Ok(match manager {
            VcsManager::Svn => Box::new(Svn::new(default_branch, working_dir)),
            _ => Box::new(Git::new(default_branch, working_dir)?),
        })
    }
}
