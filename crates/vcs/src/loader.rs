use crate::errors::VcsError;
use crate::git::Git;
use crate::svn::Svn;
use crate::vcs::Vcs;
use moon_config::{VcsManager, WorkspaceConfig};
use std::path::Path;

pub struct VcsLoader {}

impl VcsLoader {
    pub fn load(
        config: &WorkspaceConfig,
        working_dir: &Path,
    ) -> Result<Box<dyn Vcs + Send + Sync>, VcsError> {
        let vcs_config = &config.vcs;
        let manager = &vcs_config.manager;
        let default_branch = &vcs_config.default_branch;

        Ok(match manager {
            VcsManager::Svn => Box::new(Svn::new(default_branch, working_dir)),
            _ => Box::new(Git::new(default_branch, working_dir)?),
        })
    }
}
