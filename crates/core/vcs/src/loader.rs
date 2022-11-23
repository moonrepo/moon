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

        Ok(match vcs_config.manager {
            VcsManager::Svn => Box::new(Svn::load(vcs_config, working_dir)),
            _ => Box::new(Git::load(vcs_config, working_dir)?),
        })
    }
}
